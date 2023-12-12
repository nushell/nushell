use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use nu_cli::eval_source;
use nu_parser::parse;
use nu_plugin::{EncodingType, PluginResponse};
use nu_protocol::{engine::EngineState, PipelineData, Span, Value};
use nu_utils::{get_default_config, get_default_env};
use std::path::{Path, PathBuf};

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
    let home_path = if let Some(path) = nu_path::home_dir() {
        let canon_home_path = canonicalize_path(engine_state, &path);
        canon_home_path
    } else {
        std::path::PathBuf::new()
    };
    home_path
}

// FIXME: All benchmarks live in this 1 file to speed up build times when benchmarking.
// When the *_benchmarks functions were in different files, `cargo bench` would build
// an executable for every single one - incredibly slowly. Would be nice to figure out
// a way to split things up again.

fn parser_benchmarks(c: &mut Criterion) {
    let mut engine_state = load_bench_commands();
    let home_path = get_home_path(&engine_state);

    // parsing config.nu breaks without PWD set, so set a valid path
    engine_state.add_env_var(
        "PWD".into(),
        Value::string(home_path.to_string_lossy(), Span::test_data()),
    );

    let default_env = get_default_env().as_bytes();
    c.bench_function("parse_default_env_file", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_env, false),
            BatchSize::SmallInput,
        )
    });

    let default_config = get_default_config().as_bytes();
    c.bench_function("parse_default_config_file", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| parse(&mut working_set, None, default_config, false),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("eval default_env.nu", |b| {
        b.iter(|| {
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_env().as_bytes(),
                "default_env.nu",
                PipelineData::empty(),
                false,
            )
        })
    });

    c.bench_function("eval default_config.nu", |b| {
        b.iter(|| {
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_config().as_bytes(),
                "default_config.nu",
                PipelineData::empty(),
                false,
            )
        })
    });
}

fn eval_benchmarks(c: &mut Criterion) {
    let mut engine_state = load_bench_commands();
    let home_path = get_home_path(&engine_state);

    // parsing config.nu breaks without PWD set, so set a valid path
    engine_state.add_env_var(
        "PWD".into(),
        Value::string(home_path.to_string_lossy(), Span::test_data()),
    );

    c.bench_function("eval default_env.nu", |b| {
        b.iter(|| {
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_env().as_bytes(),
                "default_env.nu",
                PipelineData::empty(),
                false,
            )
        })
    });

    c.bench_function("eval default_config.nu", |b| {
        b.iter(|| {
            let mut stack = nu_protocol::engine::Stack::new();
            eval_source(
                &mut engine_state,
                &mut stack,
                get_default_config().as_bytes(),
                "default_config.nu",
                PipelineData::empty(),
                false,
            )
        })
    });
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

fn encoding_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encoding");
    let test_cnt_pairs = [(100, 5), (100, 15), (10000, 5), (10000, 15)];
    for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
        for fmt in ["json", "msgpack"] {
            group.bench_function(&format!("{fmt} encode {row_cnt} * {col_cnt}"), |b| {
                let mut res = vec![];
                let test_data =
                    PluginResponse::Value(Box::new(encoding_test_data(row_cnt, col_cnt)));
                let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
                b.iter(|| encoder.encode_response(&test_data, &mut res))
            });
        }
    }
    group.finish();
}

fn decoding_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Decoding");
    let test_cnt_pairs = [(100, 5), (100, 15), (10000, 5), (10000, 15)];
    for (row_cnt, col_cnt) in test_cnt_pairs.into_iter() {
        for fmt in ["json", "msgpack"] {
            group.bench_function(&format!("{fmt} decode for {row_cnt} * {col_cnt}"), |b| {
                let mut res = vec![];
                let test_data =
                    PluginResponse::Value(Box::new(encoding_test_data(row_cnt, col_cnt)));
                let encoder = EncodingType::try_from_bytes(fmt.as_bytes()).unwrap();
                encoder.encode_response(&test_data, &mut res).unwrap();
                let mut binary_data = std::io::Cursor::new(res);
                b.iter(|| {
                    binary_data.set_position(0);
                    encoder.decode_response(&mut binary_data)
                })
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    parser_benchmarks,
    eval_benchmarks,
    encoding_benchmarks,
    decoding_benchmarks
);
criterion_main!(benches);
