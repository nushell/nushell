use nu_cli::{eval_source, evaluate_commands};
use nu_plugin_core::{Encoder, EncodingType};
use nu_plugin_protocol::{PluginCallResponse, PluginOutput};

use nu_protocol::{
    engine::{EngineState, Stack},
    eval_const::create_nu_constant,
    PipelineData, Span, Spanned, Value, NU_VARIABLE_ID,
};
use nu_std::load_standard_library;
use nu_utils::{get_default_config, get_default_env};
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use std::hint::black_box;

use tango_bench::{benchmark_fn, tango_benchmarks, tango_main, IntoBenchmarks};

fn load_bench_commands() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

fn canonicalize_path(engine_state: &EngineState, path: &Path) -> PathBuf {
    #[allow(deprecated)]
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
        false,
    )
    .unwrap();

    (stack, engine)
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

fn bench_command(
    name: &str,
    command: &str,
    stack: Stack,
    engine: EngineState,
) -> impl IntoBenchmarks {
    let commands = Spanned {
        span: Span::unknown(),
        item: command.to_string(),
    };
    [benchmark_fn(name, move |b| {
        let commands = commands.clone();
        let stack = stack.clone();
        let engine = engine.clone();
        b.iter(move || {
            let mut stack = stack.clone();
            let mut engine = engine.clone();
            black_box(
                evaluate_commands(
                    &commands,
                    &mut engine,
                    &mut stack,
                    PipelineData::empty(),
                    None,
                    false,
                )
                .unwrap(),
            );
        })
    })]
}

fn bench_eval_source(
    name: &str,
    fname: String,
    source: Vec<u8>,
    stack: Stack,
    engine: EngineState,
) -> impl IntoBenchmarks {
    [benchmark_fn(name, move |b| {
        let stack = stack.clone();
        let engine = engine.clone();
        let fname = fname.clone();
        let source = source.clone();
        b.iter(move || {
            let mut stack = stack.clone();
            let mut engine = engine.clone();
            let fname: &str = &fname.clone();
            let source: &[u8] = &source.clone();
            black_box(eval_source(
                &mut engine,
                &mut stack,
                source,
                fname,
                PipelineData::empty(),
                false,
            ));
        })
    })]
}

/// Load the standard library into the engine.
fn bench_load_standard_lib() -> impl IntoBenchmarks {
    [benchmark_fn("load_standard_lib", move |b| {
        let engine = setup_engine();
        b.iter(move || {
            let mut engine = engine.clone();
            load_standard_library(&mut engine)
        })
    })]
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

fn bench_record_create(n: i32) -> impl IntoBenchmarks {
    bench_command(
        &format!("record_create_{n}"),
        &create_flat_record_string(n),
        Stack::new(),
        setup_engine(),
    )
}

fn bench_record_flat_access(n: i32) -> impl IntoBenchmarks {
    let setup_command = create_flat_record_string(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        &format!("record_flat_access_{n}"),
        "$record.col_0 | ignore",
        stack,
        engine,
    )
}

fn bench_record_nested_access(n: i32) -> impl IntoBenchmarks {
    let setup_command = create_nested_record_string(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    let nested_access = ".col".repeat(n as usize);
    bench_command(
        &format!("record_nested_access_{n}"),
        &format!("$record{} | ignore", nested_access),
        stack,
        engine,
    )
}

fn bench_table_create(n: i32) -> impl IntoBenchmarks {
    bench_command(
        &format!("table_create_{n}"),
        &create_example_table_nrows(n),
        Stack::new(),
        setup_engine(),
    )
}

fn bench_table_get(n: i32) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        &format!("table_get_{n}"),
        "$table | get bar | math sum | ignore",
        stack,
        engine,
    )
}

fn bench_table_select(n: i32) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        &format!("table_select_{n}"),
        "$table | select foo baz | ignore",
        stack,
        engine,
    )
}

fn bench_eval_interleave(n: i32) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        &format!("eval_interleave_{n}"),
        &format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_interleave_with_ctrlc(n: i32) -> impl IntoBenchmarks {
    let mut engine = setup_engine();
    engine.ctrlc = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
        false,
    )));
    let stack = Stack::new();
    bench_command(
        &format!("eval_interleave_with_ctrlc_{n}"),
        &format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_for(n: i32) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        &format!("eval_for_{n}"),
        &format!("(for $x in (1..{n}) {{ 1 }}) | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_each(n: i32) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        &format!("eval_each_{n}"),
        &format!("(1..{n}) | each {{|_| 1 }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_par_each(n: i32) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        &format!("eval_par_each_{n}"),
        &format!("(1..{}) | par-each -t 2 {{|_| 1 }} | ignore", n),
        stack,
        engine,
    )
}

fn bench_eval_default_config() -> impl IntoBenchmarks {
    let default_env = get_default_config().as_bytes().to_vec();
    let fname = "default_config.nu".to_string();
    bench_eval_source(
        "eval_default_config",
        fname,
        default_env,
        Stack::new(),
        setup_engine(),
    )
}

fn bench_eval_default_env() -> impl IntoBenchmarks {
    let default_env = get_default_env().as_bytes().to_vec();
    let fname = "default_env.nu".to_string();
    bench_eval_source(
        "eval_default_env",
        fname,
        default_env,
        Stack::new(),
        setup_engine(),
    )
}

fn encode_json(row_cnt: usize, col_cnt: usize) -> impl IntoBenchmarks {
    let test_data = Rc::new(PluginOutput::CallResponse(
        0,
        PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
    ));
    let encoder = Rc::new(EncodingType::try_from_bytes(b"json").unwrap());

    [benchmark_fn(
        format!("encode_json_{}_{}", row_cnt, col_cnt),
        move |b| {
            let encoder = encoder.clone();
            let test_data = test_data.clone();
            b.iter(move || {
                let mut res = Vec::new();
                encoder.encode(&*test_data, &mut res).unwrap();
            })
        },
    )]
}

fn encode_msgpack(row_cnt: usize, col_cnt: usize) -> impl IntoBenchmarks {
    let test_data = Rc::new(PluginOutput::CallResponse(
        0,
        PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
    ));
    let encoder = Rc::new(EncodingType::try_from_bytes(b"msgpack").unwrap());

    [benchmark_fn(
        format!("encode_msgpack_{}_{}", row_cnt, col_cnt),
        move |b| {
            let encoder = encoder.clone();
            let test_data = test_data.clone();
            b.iter(move || {
                let mut res = Vec::new();
                encoder.encode(&*test_data, &mut res).unwrap();
            })
        },
    )]
}

fn decode_json(row_cnt: usize, col_cnt: usize) -> impl IntoBenchmarks {
    let test_data = PluginOutput::CallResponse(
        0,
        PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
    );
    let encoder = EncodingType::try_from_bytes(b"json").unwrap();
    let mut res = vec![];
    encoder.encode(&test_data, &mut res).unwrap();

    [benchmark_fn(
        format!("decode_json_{}_{}", row_cnt, col_cnt),
        move |b| {
            let res = res.clone();
            b.iter(move || {
                let mut binary_data = std::io::Cursor::new(res.clone());
                binary_data.set_position(0);
                let _: Result<Option<PluginOutput>, _> =
                    black_box(encoder.decode(&mut binary_data));
            })
        },
    )]
}

fn decode_msgpack(row_cnt: usize, col_cnt: usize) -> impl IntoBenchmarks {
    let test_data = PluginOutput::CallResponse(
        0,
        PluginCallResponse::value(encoding_test_data(row_cnt, col_cnt)),
    );
    let encoder = EncodingType::try_from_bytes(b"msgpack").unwrap();
    let mut res = vec![];
    encoder.encode(&test_data, &mut res).unwrap();

    [benchmark_fn(
        format!("decode_msgpack_{}_{}", row_cnt, col_cnt),
        move |b| {
            let res = res.clone();
            b.iter(move || {
                let mut binary_data = std::io::Cursor::new(res.clone());
                binary_data.set_position(0);
                let _: Result<Option<PluginOutput>, _> =
                    black_box(encoder.decode(&mut binary_data));
            })
        },
    )]
}

tango_benchmarks!(
    bench_load_standard_lib(),
    // Data types
    // Record
    bench_record_create(1),
    bench_record_create(10),
    bench_record_create(100),
    bench_record_create(1_000),
    bench_record_flat_access(1),
    bench_record_flat_access(10),
    bench_record_flat_access(100),
    bench_record_flat_access(1_000),
    bench_record_nested_access(1),
    bench_record_nested_access(2),
    bench_record_nested_access(4),
    bench_record_nested_access(8),
    bench_record_nested_access(16),
    bench_record_nested_access(32),
    bench_record_nested_access(64),
    bench_record_nested_access(128),
    // Table
    bench_table_create(1),
    bench_table_create(10),
    bench_table_create(100),
    bench_table_create(1_000),
    bench_table_get(1),
    bench_table_get(10),
    bench_table_get(100),
    bench_table_get(1_000),
    bench_table_select(1),
    bench_table_select(10),
    bench_table_select(100),
    bench_table_select(1_000),
    // Eval
    // Interleave
    bench_eval_interleave(100),
    bench_eval_interleave(1_000),
    bench_eval_interleave(10_000),
    bench_eval_interleave_with_ctrlc(100),
    bench_eval_interleave_with_ctrlc(1_000),
    bench_eval_interleave_with_ctrlc(10_000),
    // For
    bench_eval_for(1),
    bench_eval_for(10),
    bench_eval_for(100),
    bench_eval_for(1_000),
    bench_eval_for(10_000),
    // Each
    bench_eval_each(1),
    bench_eval_each(10),
    bench_eval_each(100),
    bench_eval_each(1_000),
    bench_eval_each(10_000),
    // Par-Each
    bench_eval_par_each(1),
    bench_eval_par_each(10),
    bench_eval_par_each(100),
    bench_eval_par_each(1_000),
    bench_eval_par_each(10_000),
    // Config
    bench_eval_default_config(),
    // Env
    bench_eval_default_env(),
    // Encode
    // Json
    encode_json(100, 5),
    encode_json(10000, 15),
    // MsgPack
    encode_msgpack(100, 5),
    encode_msgpack(10000, 15),
    // Decode
    // Json
    decode_json(100, 5),
    decode_json(10000, 15),
    // MsgPack
    decode_msgpack(100, 5),
    decode_msgpack(10000, 15)
);

tango_main!();
