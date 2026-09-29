//! Pickles made in one engine state and loaded into another.
//!
//! The loading engine gets one extra declaration, file, variable, block and module before
//! anything else, so every id and span the program uses moves. A pickle that ran without being
//! relinked would call the wrong commands or fail to find its blocks.

use nu_engine::eval_block;
use nu_parser::{parse, pickle};
use nu_protocol::{
    Module, PipelineData, Signature, Span, Type, Value,
    ast::Block,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateDelta, StateWorkingSet},
};
use nu_test_support::prelude::*;
use std::{path::Path, sync::Arc};

const PROGRAM: &str = r#"
    use std/assert
    module inner {
        export def twice [x: int] { $x * 2 }
        export const K = 10
    }
    use inner [twice K]
    def --env mark [] { $env.PICKLED = "yes" }
    alias dbl = twice
    const base = 5
    let offset = 1
    let adder = {|x| $x + $offset }
    mark
    assert equal (dbl 2) 4
    {
        closure: (do $adder $base)
        each: ([1 2 3] | each {|x| twice $x })
        matched: (match [1 2] { [$a, $b] => ($a + $b), _ => 0 })
        const: $K
        env: $env.PICKLED
        caught: (try { error make {msg: "boom"} } catch {|e| $e.msg })
        filtered: ([1 2 3 4] | where $it > 2)
        help: (twice --help)
    }
"#;

/// The default command context plus the standard library, optionally with one of everything
/// added in front so that all later ids and spans are shifted.
fn engine(shifted: bool) -> Result<EngineState> {
    let mut engine_state = nu_cmd_lang::create_default_context();
    if shifted {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Signature::new("pickle-shift").predeclare());
        let _ = working_set.add_file("pickle-shift.nu", b"# moves every later span");
        working_set.add_variable(b"$pickle_shift".to_vec(), Span::unknown(), Type::Any, false);
        working_set.add_block(Arc::new(Block::new()));
        working_set.add_module(
            "pickle-shift",
            Module::new(b"pickle-shift".to_vec()),
            vec![],
        );
        engine_state.merge_delta(working_set.render())?;
    }
    let mut engine_state = nu_command::add_shell_command_context(engine_state);
    nu_std::load_standard_library(&mut engine_state).expect("std library loads");
    Ok(engine_state)
}

/// Parse `code` into `engine_state` for good, like a config file would.
fn define(engine_state: &mut EngineState, code: &str) -> Result {
    let mut working_set = StateWorkingSet::new(engine_state);
    parse(&mut working_set, None, code.as_bytes(), false);
    assert!(working_set.parse_errors.is_empty());
    engine_state.merge_delta(working_set.render())?;
    Ok(())
}

/// Load `pickled` into `engine_state` expecting it to be refused, and return the message and help.
fn load_error(engine_state: &EngineState, pickled: &[u8]) -> String {
    let mut working_set = StateWorkingSet::new(engine_state);
    match pickle::load(&mut working_set, pickled, Path::new("")) {
        Ok(_) => panic!("the pickle loads"),
        Err(ShellError::LabeledError(err)) => {
            format!("{}\n{}", err.msg, err.help.unwrap_or_default())
        }
        Err(err) => err.to_string(),
    }
}

fn pickle_program(engine_state: &EngineState, code: &str) -> Result<Vec<u8>> {
    let mut working_set = StateWorkingSet::new(engine_state);
    working_set.standalone = true;
    let block = parse(&mut working_set, Some("program.nu"), code.as_bytes(), false);
    assert!(working_set.parse_errors.is_empty());
    Ok(pickle::save(&working_set, &block, Path::new(""))?)
}

fn eval(engine_state: &mut EngineState, delta: StateDelta, block: &Block) -> Result<Value> {
    engine_state.merge_delta(delta)?;
    let data = eval_block::<WithoutDebug>(
        engine_state,
        &mut Stack::new(),
        block,
        PipelineData::empty(),
    )?;
    Ok(data.body.into_value(Span::unknown())?)
}

#[test]
fn pickled_program_runs_like_source_in_shifted_engine() -> Result {
    let pickled = pickle_program(&engine(false)?, PROGRAM)?;

    let mut engine_state = engine(true)?;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = pickle::load(&mut working_set, &pickled, Path::new(""))?;
    let delta = working_set.render();
    let from_pickle = eval(&mut engine_state, delta, &block)?;

    let mut engine_state = engine(true)?;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = parse(
        &mut working_set,
        Some("program.nu"),
        PROGRAM.as_bytes(),
        false,
    );
    let delta = working_set.render();
    let from_source = eval(&mut engine_state, delta, &block)?;

    assert_eq!(from_pickle, from_source);
    let record = from_pickle.into_record()?;
    assert_eq!(record.get("closure"), Some(&Value::test_int(6)));
    assert_eq!(
        record.get("each"),
        Some(&Value::test_list(vec![
            Value::test_int(2),
            Value::test_int(4),
            Value::test_int(6)
        ]))
    );
    assert_eq!(record.get("matched"), Some(&Value::test_int(3)));
    assert_eq!(record.get("const"), Some(&Value::test_int(10)));
    assert_eq!(record.get("env"), Some(&Value::test_string("yes")));
    assert_eq!(record.get("caught"), Some(&Value::test_string("boom")));
    Ok(())
}

#[test]
fn missing_command_refuses_to_load() -> Result {
    let mut producer = engine(false)?;
    define(&mut producer, "def pickle-only [] { 1 }")?;
    let pickled = pickle_program(&producer, "pickle-only")?;
    assert_contains(
        "`pickle-only` is not defined",
        load_error(&engine(true)?, &pickled),
    );
    Ok(())
}

#[test]
fn command_links_by_the_name_the_program_called() -> Result {
    let mut producer = engine(false)?;
    define(&mut producer, "module m { export def foo [] { 1 } }; use m")?;
    let pickled = pickle_program(&producer, "m foo")?;

    // Same bare name and signature, but it isn't `m foo`.
    let mut consumer = engine(true)?;
    define(&mut consumer, "def foo [] { 2 }")?;
    assert_contains("`m foo` is not defined", load_error(&consumer, &pickled));
    Ok(())
}

#[test]
fn changed_signature_refuses_to_load() -> Result {
    let mut producer = engine(false)?;
    define(&mut producer, "def pickle-cmd [x: int] { $x }")?;
    let pickled = pickle_program(&producer, "pickle-cmd 1")?;

    let mut consumer = engine(true)?;
    define(&mut consumer, "def pickle-cmd [x: string] { $x }")?;
    assert_contains(
        "`pickle-cmd` has a different signature now",
        load_error(&consumer, &pickled),
    );
    Ok(())
}

#[test]
fn damaged_pickle_is_an_error() -> Result {
    let pickled = pickle_program(&engine(false)?, PROGRAM)?;
    let engine_state = engine(false)?;

    let mut flipped = pickled.clone();
    flipped[pickled.len() / 2] ^= 1;
    for damaged in [&pickled[..pickled.len() / 2], flipped.as_slice()] {
        assert_contains("pickle is damaged", load_error(&engine_state, damaged));
    }
    Ok(())
}
