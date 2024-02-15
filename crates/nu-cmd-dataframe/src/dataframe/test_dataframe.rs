use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::{
    engine::{Command, EngineState, Stack, StateWorkingSet},
    Example, PipelineData, Span,
};

use super::eager::{SchemaDF, ToDataFrame};
use super::expressions::ExprCol;
use super::lazy::{LazyCollect, ToLazyFrame};
use nu_cmd_lang::Let;

pub fn test_dataframe(cmds: Vec<Box<dyn Command + 'static>>) {
    if cmds.is_empty() {
        panic!("Empty commands vector")
    }

    // The first element in the cmds vector must be the one tested
    let examples = cmds[0].examples();
    let mut engine_state = build_test_engine_state(cmds.clone());

    for example in examples {
        test_dataframe_example(&mut engine_state, &example);
    }
}

pub fn build_test_engine_state(cmds: Vec<Box<dyn Command + 'static>>) -> Box<EngineState> {
    let mut engine_state = Box::new(EngineState::new());

    let delta = {
        // Base functions that are needed for testing
        // Try to keep this working set small to keep tests running as fast as possible
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(Let));
        working_set.add_decl(Box::new(ToDataFrame));
        working_set.add_decl(Box::new(ToLazyFrame));
        working_set.add_decl(Box::new(LazyCollect));
        working_set.add_decl(Box::new(ExprCol));
        working_set.add_decl(Box::new(SchemaDF));

        // Adding the command that is being tested to the working set
        for cmd in cmds.clone() {
            working_set.add_decl(cmd);
        }

        working_set.render()
    };

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    engine_state
}

pub fn test_dataframe_example(engine_state: &mut Box<EngineState>, example: &Example) {
    // Skip tests that don't have results to compare to
    if example.result.is_none() {
        return;
    }

    let start = std::time::Instant::now();

    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(engine_state);
        let output = parse(&mut working_set, None, example.example.as_bytes(), false);

        if let Some(err) = working_set.parse_errors.first() {
            panic!("test parse error in `{}`: {:?}", example.example, err)
        }

        (output, working_set.render())
    };

    engine_state
        .merge_delta(delta)
        .expect("Error merging delta");

    let mut stack = Stack::new();

    let result = eval_block(
        engine_state,
        &mut stack,
        &block,
        PipelineData::empty(),
        true,
        true,
    )
    .unwrap_or_else(|err| panic!("test eval error in `{}`: {:?}", example.example, err))
    .into_value(Span::test_data());

    println!("input: {}", example.example);
    println!("result: {result:?}");
    println!("done: {:?}", start.elapsed());

    // Note. Value implements PartialEq for Bool, Int, Float, String and Block
    // If the command you are testing requires to compare another case, then
    // you need to define its equality in the Value struct
    if let Some(expected) = example.result.clone() {
        if result != expected {
            panic!("the example result is different to expected value: {result:?} != {expected:?}")
        }
    }
}
