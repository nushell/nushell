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

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

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
    )
    .unwrap();

    (stack, engine)
}

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

// generate a new table data with `row_cnt` rows, `col_cnt` columns.
fn encoding_test_data(row_cnt: usize, col_cnt: usize) -> Value {
    let record = Value::test_record(
        (0..col_cnt)
            .map(|x| (format!("col_{x}"), Value::test_int(x as i64)))
            .collect(),
    );

    Value::list(vec![record; row_cnt], Span::test_data())
}

fn span_command(command: &str) -> Spanned<String> {
    Spanned {
        span: Span::unknown(),
        item: command.to_string(),
    }
}

// FIXME: All benchmarks live in this 1 file to speed up build times when benchmarking.
// When the *_benchmarks functions were in different files, `cargo bench` would build
// an executable for every single one - incredibly slowly. Would be nice to figure out
// a way to split things up again.

fn bench_load_standard_lib(c: &mut Criterion) {
    c.bench_function("load_standard_lib", |b| {
        let engine = setup_engine();
        b.iter_batched(
            || {
                return engine.clone();
            },
            |mut engine| {
                load_standard_library(&mut engine).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_create_records(b: &mut Criterion) {
    let mut g = b.benchmark_group("create_records");
    for n in [1, 10, 100, 1000].iter() {
        let command = span_command(&create_flat_record_string(*n));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), setup_engine()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_flat_record(b: &mut Criterion) {
    let mut g = b.benchmark_group("flat_access");
    for n in [1, 10, 100, 1000].iter() {
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || setup_stack_and_engine_from_command(&create_flat_record_string(*n)),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &span_command("$record.col_0 | ignore"),
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_nested_record_access(b: &mut Criterion) {
    let mut g = b.benchmark_group("nest_access");
    for n in [1, 2, 4, 8, 16, 32, 64, 128].iter() {
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            let nested_access = ".col".repeat(*n as usize);
            let nested_access = format!("$record{} | ignore", nested_access);
            b.iter_batched(
                || setup_stack_and_engine_from_command(&create_nested_record_string(*n)),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &span_command(&nested_access),
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_create_table(b: &mut Criterion) {
    let mut g = b.benchmark_group("create_table");
    for n in [1, 10, 100, 1000].iter() {
        let command = span_command(&create_example_table_nrows(*n));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), setup_engine()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_table_get(b: &mut Criterion) {
    let mut g = b.benchmark_group("table_get");
    for n in [1, 10, 100, 1000].iter() {
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || setup_stack_and_engine_from_command(&create_example_table_nrows(*n)),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &span_command("$table | get bar | math sum | ignore"),
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_table_select(b: &mut Criterion) {
    let mut g = b.benchmark_group("table_select");
    for n in [1, 10, 100, 1000].iter() {
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || setup_stack_and_engine_from_command(&create_example_table_nrows(*n)),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &span_command("$table | select foo baz | ignore"),
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_interleve(b: &mut Criterion) {
    let mut g = b.benchmark_group("interleave");
    let engine = setup_engine();
    for n in [100, 1_000, 10_000].iter() {
        let command = span_command(&format!(
            "seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"
        ));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_interleave_with_ctrlc(b: &mut Criterion) {
    let mut g = b.benchmark_group("interleave_with_ctrlc");
    let mut engine = setup_engine();
    engine.ctrlc = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    )));
    for n in [100, 1_000, 10_000].iter() {
        let command = span_command(&format!(
            "seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"
        ));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn benc_for(b: &mut Criterion) {
    let mut g = b.benchmark_group("for");
    let engine = setup_engine();

    for n in [1, 5, 10, 100, 1_000].iter() {
        let command = span_command(&format!("(for $x in (1..{n}) {{  }}) | ignore"));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn benc_each(b: &mut Criterion) {
    let mut g = b.benchmark_group("each");
    let engine = setup_engine();

    for n in [1, 5, 10, 100, 1_000].iter() {
        let command = span_command(&format!("(1..{n}) | each {{|_| 0 }} | ignore"));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn benc_par_each_1t(b: &mut Criterion) {
    let mut g = b.benchmark_group("par-each-1t");
    let engine = setup_engine();

    for n in [1, 5, 10, 100, 1_000].iter() {
        let command = span_command(&format!("(1..{n}) | par-each -t 1 {{|_| 0 }} | ignore"));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn benc_par_each_2t(b: &mut Criterion) {
    let mut g = b.benchmark_group("par-each-2t");
    let engine = setup_engine();

    for n in [1, 5, 10, 100, 1_000].iter() {
        let command = span_command(&format!("(1..{n}) | par-each -t 2 {{|_| 0 }} | ignore"));
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || (Stack::new(), engine.clone()),
                |(mut stack, mut engine)| {
                    evaluate_commands(
                        &command,
                        &mut engine,
                        &mut stack,
                        PipelineData::empty(),
                        None,
                    )
                    .unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

fn bench_parse_default_config(b: &mut Criterion) {
    let engine_state = setup_engine();
    let default_env = get_default_config().as_bytes();

    b.bench_function("parse_default_config", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| {
                parse(&mut working_set, None, default_env, false);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_parse_default_env_file(b: &mut Criterion) {
    let engine_state = setup_engine();
    let default_env = get_default_env().as_bytes();

    b.bench_function("parse_default_env_file", |b| {
        b.iter_batched(
            || nu_protocol::engine::StateWorkingSet::new(&engine_state),
            |mut working_set| {
                parse(&mut working_set, None, default_env, false);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_eval_default_env(b: &mut Criterion) {
    let engine_state = setup_engine();
    let default_env = get_default_env().as_bytes();
    let fname = "default_env.nu";

    b.bench_function("eval_default_env", |b| {
        b.iter_batched(
            || (engine_state.clone(), nu_protocol::engine::Stack::new()),
            |(mut engine_state, mut stack)| {
                eval_source(
                    &mut engine_state,
                    &mut stack,
                    default_env,
                    fname,
                    PipelineData::empty(),
                    false,
                )
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_eval_default_config(b: &mut Criterion) {
    let engine_state = setup_engine();
    let default_env = get_default_config().as_bytes();
    let fname = "default_config.nu";

    b.bench_function("eval_default_config", |b| {
        b.iter_batched(
            || (engine_state.clone(), nu_protocol::engine::Stack::new()),
            |(mut engine_state, mut stack)| {
                eval_source(
                    &mut engine_state,
                    &mut stack,
                    default_env,
                    fname,
                    PipelineData::empty(),
                    false,
                )
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_encode_json(b: &mut Criterion) {
    let mut g = b.benchmark_group("encode_json");

    for n in [(100, 5), (10000, 15)].iter() {
        let test_data =
            PluginOutput::CallResponse(0, PluginCallResponse::value(encoding_test_data(n.0, n.1)));
        let encoder = EncodingType::try_from_bytes(b"json").unwrap();

        g.bench_function(
            BenchmarkId::from_parameter(format!("{} {}", n.0, n.1)),
            |b| {
                b.iter_batched(
                    Vec::new,
                    |mut res| encoder.encode(&test_data, &mut res),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

fn bench_encode_msgpack(b: &mut Criterion) {
    let mut g = b.benchmark_group("encode_msgpack");

    for n in [(100, 5), (10000, 15)].iter() {
        let test_data =
            PluginOutput::CallResponse(0, PluginCallResponse::value(encoding_test_data(n.0, n.1)));
        let encoder = EncodingType::try_from_bytes(b"msgpack").unwrap();

        g.bench_function(
            BenchmarkId::from_parameter(format!("{} {}", n.0, n.1)),
            |b| {
                b.iter_batched(
                    Vec::new,
                    |mut res| encoder.encode(&test_data, &mut res),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

fn bench_decode_json(b: &mut Criterion) {
    let mut g = b.benchmark_group("decode_json");

    for n in [(100, 5), (10000, 15)].iter() {
        let test_data =
            PluginOutput::CallResponse(0, PluginCallResponse::value(encoding_test_data(n.0, n.1)));
        let encoder = EncodingType::try_from_bytes(b"json").unwrap();
        let mut res = vec![];
        encoder.encode(&test_data, &mut res).unwrap();

        g.bench_function(
            BenchmarkId::from_parameter(format!("{} {}", n.0, n.1)),
            |b| {
                b.iter_batched(
                    || {
                        let mut binary_data = std::io::Cursor::new(res.clone());
                        binary_data.set_position(0);
                        binary_data
                    },
                    |mut binary_data| -> Result<Option<PluginOutput>, _> {
                        encoder.decode(&mut binary_data)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

fn bench_decode_msgpack(b: &mut Criterion) {
    let mut g = b.benchmark_group("decode_msgpack");

    for n in [(100, 5), (10000, 15)].iter() {
        let test_data =
            PluginOutput::CallResponse(0, PluginCallResponse::value(encoding_test_data(n.0, n.1)));
        let encoder = EncodingType::try_from_bytes(b"msgpack").unwrap();
        let mut res = vec![];
        encoder.encode(&test_data, &mut res).unwrap();

        g.bench_function(
            BenchmarkId::from_parameter(format!("{} {}", n.0, n.1)),
            |b| {
                b.iter_batched(
                    || {
                        let mut binary_data = std::io::Cursor::new(res.clone());
                        binary_data.set_position(0);
                        binary_data
                    },
                    |mut binary_data| -> Result<Option<PluginOutput>, _> {
                        encoder.decode(&mut binary_data)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

criterion_group!(
    benches,
    bench_load_standard_lib,
    bench_create_records,
    bench_flat_record,
    bench_nested_record_access,
    bench_create_table,
    bench_table_get,
    bench_table_select,
    bench_interleve,
    bench_interleave_with_ctrlc,
    benc_for,
    benc_each,
    benc_par_each_1t,
    benc_par_each_2t,
    bench_parse_default_config,
    bench_parse_default_env_file,
    bench_eval_default_env,
    bench_eval_default_config,
    bench_encode_json,
    bench_encode_msgpack,
    bench_decode_json,
    bench_decode_msgpack
);

criterion_main!(benches);
