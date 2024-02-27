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

// FIXME: All benchmarks live in this 1 file to speed up build times when benchmarking.
// When the *_benchmarks functions were in different files, `cargo bench` would build
// an executable for every single one - incredibly slowly. Would be nice to figure out
// a way to split things up again.

mod parser_benchmarks {
    use super::*;

    fn setup() -> EngineState {
        let mut engine_state = load_bench_commands();
        let home_path = get_home_path(&engine_state);

        // parsing config.nu breaks without PWD set, so set a valid path
        engine_state.add_env_var(
            "PWD".into(),
            Value::string(home_path.to_string_lossy(), Span::test_data()),
        );

        engine_state
    }

    #[divan::bench()]
    fn parse_default_config_file(bencher: divan::Bencher) {
        let engine_state = setup();
        let default_env = get_default_config().as_bytes();

        bencher
            .with_inputs(|| nu_protocol::engine::StateWorkingSet::new(&engine_state))
            .bench_refs(|mut working_set| parse(&mut working_set, None, default_env, false))
    }

    #[divan::bench()]
    fn parse_default_env_file(bencher: divan::Bencher) {
        let engine_state = setup();
        let default_env = get_default_env().as_bytes();

        bencher
            .with_inputs(|| nu_protocol::engine::StateWorkingSet::new(&engine_state))
            .bench_refs(|mut working_set| parse(&mut working_set, None, default_env, false))
    }
}

mod eval_benchmarks {
    use super::*;

    fn setup() -> EngineState {
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

    #[divan::bench()]
    fn eval_default_env(bencher: divan::Bencher) {
        let default_env = get_default_env().as_bytes();
        let fname = "default_env.nu";
        bencher
            .with_inputs(|| (setup(), nu_protocol::engine::Stack::new()))
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
            .with_inputs(|| (setup(), nu_protocol::engine::Stack::new()))
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
            .with_inputs(|| (vec![]))
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
            .with_inputs(|| (vec![]))
            .bench_values(|mut res| encoder.encode(&test_data, &mut res))
    }
}

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

// fn parser_benchmarks(c: &mut Criterion) {
//     let mut engine_state = load_bench_commands();
//     let home_path = get_home_path(&engine_state);

//     // parsing config.nu breaks without PWD set, so set a valid path
//     engine_state.add_env_var(
//         "PWD".into(),
//         Value::string(home_path.to_string_lossy(), Span::test_data()),
//     );

//     let default_env = get_default_env().as_bytes();
//     c.bench_function("parse_default_env_file", |b| {
//         b.iter_batched(
//             || nu_protocol::engine::StateWorkingSet::new(&engine_state),
//             |mut working_set| parse(&mut working_set, None, default_env, false),
//             BatchSize::SmallInput,
//         )
//     });

//     let default_config = get_default_config().as_bytes();
//     c.bench_function("parse_default_config_file", |b| {
//         b.iter_batched(
//             || nu_protocol::engine::StateWorkingSet::new(&engine_state),
//             |mut working_set| parse(&mut working_set, None, default_config, false),
//             BatchSize::SmallInput,
//         )
//     });
// }

// fn eval_benchmarks(c: &mut Criterion) {
//     let mut engine_state = load_bench_commands();
//     let home_path = get_home_path(&engine_state);

//     // parsing config.nu breaks without PWD set, so set a valid path
//     engine_state.add_env_var(
//         "PWD".into(),
//         Value::string(home_path.to_string_lossy(), Span::test_data()),
//     );

//     let nu_const = create_nu_constant(&engine_state, Span::unknown())
//         .expect("Failed to create nushell constant.");
//     engine_state.set_variable_const_val(NU_VARIABLE_ID, nu_const);

//     c.bench_function("eval default_env.nu", |b| {
//         b.iter(|| {
//             let mut stack = nu_protocol::engine::Stack::new();
//             eval_source(
//                 &mut engine_state,
//                 &mut stack,
//                 get_default_env().as_bytes(),
//                 "default_env.nu",
//                 PipelineData::empty(),
//                 false,
//             )
//         })
//     });

//     c.bench_function("eval default_config.nu", |b| {
//         b.iter(|| {
//             let mut stack = nu_protocol::engine::Stack::new();
//             eval_source(
//                 &mut engine_state,
//                 &mut stack,
//                 get_default_config().as_bytes(),
//                 "default_config.nu",
//                 PipelineData::empty(),
//                 false,
//             )
//         })
//     });
// }

// // generate a new table data with `row_cnt` rows, `col_cnt` columns.
// fn encoding_test_data(row_cnt: usize, col_cnt: usize) -> Value {
//     let record = Value::test_record(
//         (0..col_cnt)
//             .map(|x| (format!("col_{x}"), Value::test_int(x as i64)))
//             .collect(),
//     );

//     Value::list(vec![record; row_cnt], Span::test_data())
// }

// fn encoding_benchmarks(c: &mut Criterion) {
//     let mut group = c.benchmark_group("Encoding");
//     let test_cnt_pairs = [(100, 5), (10000, 15)];
//     for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
//         for fmt in ["json", "msgpack"] {
//             group.bench_function(&format!("{fmt} encode {row_cnt} * {col_cnt}"), |b| {
//                 let mut res = vec![];
//                 let test_data = PluginOutput::CallResponse(
//                     0,
//                     PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
//                 );
//                 let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
//                 b.iter(|| encoder.encode(&test_data, &mut res))
//             });
//         }
//     }
//     group.finish();
// }

// fn decoding_benchmarks(c: &mut Criterion) {
//     let mut group = c.benchmark_group("Decoding");
//     let test_cnt_pairs = [(100, 5), (10000, 15)];
//     for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
//         for fmt in ["json", "msgpack"] {
//             group.bench_function(&format!("{fmt} decode for {row_cnt} * {col_cnt}"), |b| {
//                 let mut res = vec![];
//                 let test_data = PluginOutput::CallResponse(
//                     0,
//                     PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
//                 );
//                 let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
//                 encoder.encode(&test_data, &mut res).unwrap();
//                 let mut binary_data = std::io::Cursor::new(res);
//                 b.iter(|| -> Result<Option<PluginOutput>, _> {
//                     binary_data.set_position(0);
//                     encoder.decode(&mut binary_data)
//                 })
//             });
//         }
//     }
//     group.finish();
// }

// criterion_group!(
//     benches,
//     parser_benchmarks,
//     eval_benchmarks,
//     encoding_benchmarks,
//     decoding_benchmarks
// );
// criterion_main!(benches);
