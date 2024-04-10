use nu_cli::{eval_source, evaluate_commands};
use nu_parser::parse;
use nu_plugin::{Encoder, EncodingType, PluginCallResponse, PluginOutput};
use nu_protocol::{
    engine::{EngineState, Stack},
    eval_const::create_nu_constant,
    PipelineData, Span, Spanned, Value, NU_VARIABLE_ID,
};
use nu_std::load_standard_library;
use nu_utils::{get_default_config, get_default_env};
use std::path::{Path, PathBuf};

fn main() {
    // Run registered benchmarks.
    divan::main();
}

fn load_bench_commands() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

fn canonicalize_path(engine_state: &EngineState, path: &Path) -> PathBuf {
    let cwd = engine_state.current_work_dir();

    if path.exists() {
        match nu_path::canonicalize_with(path, cwd) {
            Ok(canon_path) => canon_path,
            Err(_) => path.to_owned(),
        }
    } else {
        path.to_owned()
    }
}

fn get_home_path(engine_state: &EngineState) -> PathBuf {
    nu_path::home_dir()
        .map(|path| canonicalize_path(engine_state, &path))
        .unwrap_or_default()
}

fn setup_engine() -> EngineState {
    let mut engine_state = load_bench_commands();
    let home_path = get_home_path(&engine_state);

    // parsing config.nu breaks without PWD set, so set a valid path
    engine_state.add_env_var(
        "PWD".into(),
        Value::string(home_path.to_string_lossy(), Span::test_data()),
    );

    let nu_const = create_nu_constant(&engine_state, Span::unknown())
        .expect("Failed to create nushell constant.");
    engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);

    engine_state
}

fn bench_command(bencher: divan::Bencher, scaled_command: String) {
    bench_command_with_custom_stack_and_engine(
        bencher,
        scaled_command,
        Stack::new(),
        setup_engine(),
    )
}

fn bench_command_with_custom_stack_and_engine(
    bencher: divan::Bencher,
    scaled_command: String,
    stack: nu_protocol::engine::Stack,
    mut engine: EngineState,
) {
    load_standard_library(&mut engine).unwrap();
    let commands = Spanned {
        span: Span::unknown(),
        item: scaled_command,
    };

    bencher
        .with_inputs(|| engine.clone())
        .bench_values(|mut engine| {
            evaluate_commands(
                &commands,
                &mut engine,
                &mut stack.clone(),
                PipelineData::empty(),
                None,
                false,
            )
            .unwrap();
        })
}

fn setup_stack_and_engine_from_command(command: &str) -> (Stack, EngineState) {
    let mut engine = setup_engine();
    let commands = Spanned {
        span: Span::unknown(),
        item: command.to_string(),
    };

    let mut stack = Stack::new();
    evaluate_commands(
        &commands,
        &mut engine,
        &mut stack,
        PipelineData::empty(),
        None,
        false,
    )
    .unwrap();

    (stack, engine)
}

// FIXME: All benchmarks live in this 1 file to speed up build times when benchmarking.
// When the *_benchmarks functions were in different files, `cargo bench` would build
// an executable for every single one - incredibly slowly. Would be nice to figure out
// a way to split things up again.

#[divan::bench]
fn load_standard_lib(bencher: divan::Bencher) {
    let engine = setup_engine();
    bencher
        .with_inputs(|| engine.clone())
        .bench_values(|mut engine| {
            load_standard_library(&mut engine).unwrap();
        })
}

#[divan::bench_group]
mod record {

    use super::*;

    fn create_flat_record_string(n: i32) -> String {
        let mut s = String::from("let record = {");
        for i in 0..n {
            s.push_str(&format!("col_{}: {}", i, i));
            if i < n - 1 {
                s.push_str(", ");
            }
        }
        s.push('}');
        s
    }

    fn create_nested_record_string(depth: i32) -> String {
        let mut s = String::from("let record = {");
        for _ in 0..depth {
            s.push_str("col: {");
        }
        s.push_str("col_final: 0");
        for _ in 0..depth {
            s.push('}');
        }
        s.push('}');
        s
    }

    #[divan::bench(args = [1, 10, 100, 1000])]
    fn create(bencher: divan::Bencher, n: i32) {
        bench_command(bencher, create_flat_record_string(n));
    }

    #[divan::bench(args = [1, 10, 100, 1000])]
    fn flat_access(bencher: divan::Bencher, n: i32) {
        let (stack, engine) = setup_stack_and_engine_from_command(&create_flat_record_string(n));
        bench_command_with_custom_stack_and_engine(
            bencher,
            "$record.col_0 | ignore".to_string(),
            stack,
            engine,
        );
    }

    #[divan::bench(args = [1, 2, 4, 8, 16, 32, 64, 128])]
    fn nest_access(bencher: divan::Bencher, depth: i32) {
        let (stack, engine) =
            setup_stack_and_engine_from_command(&create_nested_record_string(depth));
        let nested_access = ".col".repeat(depth as usize);
        bench_command_with_custom_stack_and_engine(
            bencher,
            format!("$record{} | ignore", nested_access),
            stack,
            engine,
        );
    }
}

#[divan::bench_group]
mod table {

    use super::*;

    fn create_example_table_nrows(n: i32) -> String {
        let mut s = String::from("let table = [[foo bar baz]; ");
        for i in 0..n {
            s.push_str(&format!("[0, 1, {i}]"));
            if i < n - 1 {
                s.push_str(", ");
            }
        }
        s.push(']');
        s
    }

    #[divan::bench(args = [1, 10, 100, 1000])]
    fn create(bencher: divan::Bencher, n: i32) {
        bench_command(bencher, create_example_table_nrows(n));
    }

    #[divan::bench(args = [1, 10, 100, 1000])]
    fn get(bencher: divan::Bencher, n: i32) {
        let (stack, engine) = setup_stack_and_engine_from_command(&create_example_table_nrows(n));
        bench_command_with_custom_stack_and_engine(
            bencher,
            "$table | get bar | math sum | ignore".to_string(),
            stack,
            engine,
        );
    }

    #[divan::bench(args = [1, 10, 100, 1000])]
    fn select(bencher: divan::Bencher, n: i32) {
        let (stack, engine) = setup_stack_and_engine_from_command(&create_example_table_nrows(n));
        bench_command_with_custom_stack_and_engine(
            bencher,
            "$table | select foo baz | ignore".to_string(),
            stack,
            engine,
        );
    }
}

#[divan::bench_group]
mod eval_commands {
    use super::*;

    #[divan::bench(args = [100, 1_000, 10_000])]
    fn interleave(bencher: divan::Bencher, n: i32) {
        bench_command(
            bencher,
            format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        )
    }

    #[divan::bench(args = [100, 1_000, 10_000])]
    fn interleave_with_ctrlc(bencher: divan::Bencher, n: i32) {
        let mut engine = setup_engine();
        engine.ctrlc = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        )));
        load_standard_library(&mut engine).unwrap();
        let commands = Spanned {
            span: Span::unknown(),
            item: format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        };

        bencher
            .with_inputs(|| engine.clone())
            .bench_values(|mut engine| {
                evaluate_commands(
                    &commands,
                    &mut engine,
                    &mut nu_protocol::engine::Stack::new(),
                    PipelineData::empty(),
                    None,
                    false,
                )
                .unwrap();
            })
    }

    #[divan::bench(args = [1, 5, 10, 100, 1_000])]
    fn for_range(bencher: divan::Bencher, n: i32) {
        bench_command(bencher, format!("(for $x in (1..{}) {{ sleep 50ns }})", n))
    }

    #[divan::bench(args = [1, 5, 10, 100, 1_000])]
    fn each(bencher: divan::Bencher, n: i32) {
        bench_command(
            bencher,
            format!("(1..{}) | each {{|_| sleep 50ns }} | ignore", n),
        )
    }

    #[divan::bench(args = [1, 5, 10, 100, 1_000])]
    fn par_each_1t(bencher: divan::Bencher, n: i32) {
        bench_command(
            bencher,
            format!("(1..{}) | par-each -t 1 {{|_| sleep 50ns }} | ignore", n),
        )
    }

    #[divan::bench(args = [1, 5, 10, 100, 1_000])]
    fn par_each_2t(bencher: divan::Bencher, n: i32) {
        bench_command(
            bencher,
            format!("(1..{}) | par-each -t 2 {{|_| sleep 50ns }} | ignore", n),
        )
    }
}

#[divan::bench_group()]
mod parser_benchmarks {
    use super::*;

    #[divan::bench()]
    fn parse_default_config_file(bencher: divan::Bencher) {
        let engine_state = setup_engine();
        let default_env = get_default_config().as_bytes();

        bencher
            .with_inputs(|| nu_protocol::engine::StateWorkingSet::new(&engine_state))
            .bench_refs(|working_set| parse(working_set, None, default_env, false))
    }

    #[divan::bench()]
    fn parse_default_env_file(bencher: divan::Bencher) {
        let engine_state = setup_engine();
        let default_env = get_default_env().as_bytes();

        bencher
            .with_inputs(|| nu_protocol::engine::StateWorkingSet::new(&engine_state))
            .bench_refs(|working_set| parse(working_set, None, default_env, false))
    }
}

#[divan::bench_group()]
mod eval_benchmarks {
    use super::*;

    #[divan::bench()]
    fn eval_default_env(bencher: divan::Bencher) {
        let default_env = get_default_env().as_bytes();
        let fname = "default_env.nu";
        bencher
            .with_inputs(|| (setup_engine(), nu_protocol::engine::Stack::new()))
            .bench_values(|(mut engine_state, mut stack)| {
                eval_source(
                    &mut engine_state,
                    &mut stack,
                    default_env,
                    fname,
                    PipelineData::empty(),
                    false,
                )
            })
    }

    #[divan::bench()]
    fn eval_default_config(bencher: divan::Bencher) {
        let default_env = get_default_config().as_bytes();
        let fname = "default_config.nu";
        bencher
            .with_inputs(|| (setup_engine(), nu_protocol::engine::Stack::new()))
            .bench_values(|(mut engine_state, mut stack)| {
                eval_source(
                    &mut engine_state,
                    &mut stack,
                    default_env,
                    fname,
                    PipelineData::empty(),
                    false,
                )
            })
    }
}

// generate a new table data with `row_cnt` rows, `col_cnt` columns.
fn encoding_test_data(row_cnt: usize, col_cnt: usize) -> Value {
    let record = Value::test_record(
        (0..col_cnt)
            .map(|x| (format!("col_{x}"), Value::test_int(x as i64)))
            .collect(),
    );

    Value::list(vec![record; row_cnt], Span::test_data())
}

#[divan::bench_group()]
mod encoding_benchmarks {
    use super::*;

    #[divan::bench(args = [(100, 5), (10000, 15)])]
    fn json_encode(bencher: divan::Bencher, (row_cnt, col_cnt): (usize, usize)) {
        let test_data = PluginOutput::CallResponse(
            0,
            PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
        );
        let encoder = EncodingType::try_from_bytes(b"json").unwrap();
        bencher
            .with_inputs(Vec::new)
            .bench_values(|mut res| encoder.encode(&test_data, &mut res))
    }

    #[divan::bench(args = [(100, 5), (10000, 15)])]
    fn msgpack_encode(bencher: divan::Bencher, (row_cnt, col_cnt): (usize, usize)) {
        let test_data = PluginOutput::CallResponse(
            0,
            PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
        );
        let encoder = EncodingType::try_from_bytes(b"msgpack").unwrap();
        bencher
            .with_inputs(Vec::new)
            .bench_values(|mut res| encoder.encode(&test_data, &mut res))
    }
}

#[divan::bench_group()]
mod decoding_benchmarks {
    use super::*;

    #[divan::bench(args = [(100, 5), (10000, 15)])]
    fn json_decode(bencher: divan::Bencher, (row_cnt, col_cnt): (usize, usize)) {
        let test_data = PluginOutput::CallResponse(
            0,
            PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
        );
        let encoder = EncodingType::try_from_bytes(b"json").unwrap();
        let mut res = vec![];
        encoder.encode(&test_data, &mut res).unwrap();
        bencher
            .with_inputs(|| {
                let mut binary_data = std::io::Cursor::new(res.clone());
                binary_data.set_position(0);
                binary_data
            })
            .bench_values(|mut binary_data| -> Result<Option<PluginOutput>, _> {
                encoder.decode(&mut binary_data)
            })
    }

    #[divan::bench(args = [(100, 5), (10000, 15)])]
    fn msgpack_decode(bencher: divan::Bencher, (row_cnt, col_cnt): (usize, usize)) {
        let test_data = PluginOutput::CallResponse(
            0,
            PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
        );
        let encoder = EncodingType::try_from_bytes(b"msgpack").unwrap();
        let mut res = vec![];
        encoder.encode(&test_data, &mut res).unwrap();
        bencher
            .with_inputs(|| {
                let mut binary_data = std::io::Cursor::new(res.clone());
                binary_data.set_position(0);
                binary_data
            })
            .bench_values(|mut binary_data| -> Result<Option<PluginOutput>, _> {
                encoder.decode(&mut binary_data)
            })
    }
}
