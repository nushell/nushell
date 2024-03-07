use nu_cli::eval_source;
use nu_parser::parse;
use nu_plugin::{Encoder, EncodingType, PluginCallResponse, PluginOutput};
use nu_protocol::{
    engine::EngineState, eval_const::create_nu_constant, PipelineData, Span, Value, NU_VARIABLE_ID,
};
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

// FIXME: All benchmarks live in this 1 file to speed up build times when benchmarking.
// When the *_benchmarks functions were in different files, `cargo bench` would build
// an executable for every single one - incredibly slowly. Would be nice to figure out
// a way to split things up again.

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
