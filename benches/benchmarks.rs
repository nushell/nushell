use nu_cli::{eval_source, evaluate_commands};
use nu_plugin_core::{Encoder, EncodingType};
use nu_plugin_protocol::{PluginCallResponse, PluginOutput};
use nu_protocol::{
    PipelineData, Signals, Span, Spanned, Value,
    engine::{EngineState, Stack},
};
use nu_std::load_standard_library;
use nu_utils::ConfigFileKind;
use std::{
    fmt::Write,
    hint::black_box,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};
use tango_bench::{IntoBenchmarks, benchmark_fn, tango_benchmarks, tango_main};

fn load_bench_commands() -> EngineState {
    nu_command::add_shell_command_context(nu_cmd_lang::create_default_context())
}

fn setup_engine() -> EngineState {
    let mut engine_state = load_bench_commands();
    let cwd = std::env::current_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();

    // parsing config.nu breaks without PWD set, so set a valid path
    engine_state.add_env_var("PWD".into(), Value::string(cwd, Span::test_data()));

    engine_state.generate_nu_constant();

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
        Default::default(),
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
    name: impl Into<String>,
    command: impl Into<String> + Clone,
    stack: Stack,
    engine: EngineState,
) -> impl IntoBenchmarks {
    let commands = Spanned {
        span: Span::unknown(),
        item: command.into(),
    };
    [benchmark_fn(name, move |b| {
        let commands = commands.clone();
        let stack = stack.clone();
        let engine = engine.clone();
        b.iter(move || {
            let mut stack = stack.clone();
            let mut engine = engine.clone();
            #[allow(clippy::unit_arg)]
            black_box(
                evaluate_commands(
                    &commands,
                    &mut engine,
                    &mut stack,
                    PipelineData::empty(),
                    Default::default(),
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

/// Load all modules of standard library into the engine through a general `use`.
fn bench_load_use_standard_lib() -> impl IntoBenchmarks {
    [benchmark_fn("load_use_standard_lib", move |b| {
        // We need additional commands like `format number` for the standard library
        let engine = nu_cmd_extra::add_extra_command_context(setup_engine());
        let commands = Spanned {
            item: "use std".into(),
            span: Span::unknown(),
        };
        b.iter(move || {
            let mut engine = engine.clone();
            let mut stack = Stack::new();
            let _ = load_standard_library(&mut engine);
            evaluate_commands(
                &commands,
                &mut engine,
                &mut stack,
                PipelineData::empty(),
                Default::default(),
            )
        })
    })]
}

fn create_flat_record_string(n: usize) -> String {
    let mut s = String::from("let record = { ");
    for i in 0..n {
        write!(s, "col_{i}: {i}, ").unwrap();
    }
    s.push('}');
    s
}

fn create_nested_record_string(depth: usize) -> String {
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

fn create_example_table_nrows(n: usize) -> String {
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

fn bench_record_create(n: usize) -> impl IntoBenchmarks {
    bench_command(
        format!("record_create_{n}"),
        create_flat_record_string(n),
        Stack::new(),
        setup_engine(),
    )
}

fn bench_record_flat_access(n: usize) -> impl IntoBenchmarks {
    let setup_command = create_flat_record_string(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        format!("record_flat_access_{n}"),
        "$record.col_0 | ignore",
        stack,
        engine,
    )
}

fn bench_record_nested_access(n: usize) -> impl IntoBenchmarks {
    let setup_command = create_nested_record_string(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    let nested_access = ".col".repeat(n);
    bench_command(
        format!("record_nested_access_{n}"),
        format!("$record{nested_access} | ignore"),
        stack,
        engine,
    )
}

fn bench_record_insert(n: usize, m: usize) -> impl IntoBenchmarks {
    let setup_command = create_flat_record_string(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    let mut insert = String::from("$record");
    for i in n..(n + m) {
        write!(insert, " | insert col_{i} {i}").unwrap();
    }
    insert.push_str(" | ignore");
    bench_command(format!("record_insert_{n}_{m}"), insert, stack, engine)
}

fn bench_table_create(n: usize) -> impl IntoBenchmarks {
    bench_command(
        format!("table_create_{n}"),
        create_example_table_nrows(n),
        Stack::new(),
        setup_engine(),
    )
}

fn bench_table_get(n: usize) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        format!("table_get_{n}"),
        "$table | get bar | math sum | ignore",
        stack,
        engine,
    )
}

fn bench_table_select(n: usize) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    bench_command(
        format!("table_select_{n}"),
        "$table | select foo baz | ignore",
        stack,
        engine,
    )
}

fn bench_table_insert_row(n: usize, m: usize) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    let mut insert = String::from("$table");
    for i in n..(n + m) {
        write!(insert, " | insert {i} {{ foo: 0, bar: 1, baz: {i} }}").unwrap();
    }
    insert.push_str(" | ignore");
    bench_command(format!("table_insert_row_{n}_{m}"), insert, stack, engine)
}

fn bench_table_insert_col(n: usize, m: usize) -> impl IntoBenchmarks {
    let setup_command = create_example_table_nrows(n);
    let (stack, engine) = setup_stack_and_engine_from_command(&setup_command);
    let mut insert = String::from("$table");
    for i in 0..m {
        write!(insert, " | insert col_{i} {i}").unwrap();
    }
    insert.push_str(" | ignore");
    bench_command(format!("table_insert_col_{n}_{m}"), insert, stack, engine)
}

fn bench_eval_interleave(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        format!("eval_interleave_{n}"),
        format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_interleave_with_interrupt(n: usize) -> impl IntoBenchmarks {
    let mut engine = setup_engine();
    engine.set_signals(Signals::new(Arc::new(AtomicBool::new(false))));
    let stack = Stack::new();
    bench_command(
        format!("eval_interleave_with_interrupt_{n}"),
        format!("seq 1 {n} | wrap a | interleave {{ seq 1 {n} | wrap b }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_for(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        format!("eval_for_{n}"),
        format!("(for $x in (1..{n}) {{ 1 }}) | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_each(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        format!("eval_each_{n}"),
        format!("(1..{n}) | each {{|_| 1 }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_par_each(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        format!("eval_par_each_{n}"),
        format!("(1..{n}) | par-each -t 2 {{|_| 1 }} | ignore"),
        stack,
        engine,
    )
}

fn bench_eval_default_config() -> impl IntoBenchmarks {
    let kind = ConfigFileKind::Config;
    let default_env = kind.default().as_bytes().to_vec();
    let fname = kind.default_path().to_string();
    bench_eval_source(
        "eval_default_config",
        fname,
        default_env,
        Stack::new(),
        setup_engine(),
    )
}

fn bench_eval_default_env() -> impl IntoBenchmarks {
    let kind = ConfigFileKind::Env;
    let default_env = kind.default().as_bytes().to_vec();
    let fname = kind.default_path().to_string();
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
        format!("encode_json_{row_cnt}_{col_cnt}"),
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
        format!("encode_msgpack_{row_cnt}_{col_cnt}"),
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
        format!("decode_json_{row_cnt}_{col_cnt}"),
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
        format!("decode_msgpack_{row_cnt}_{col_cnt}"),
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
    bench_load_use_standard_lib(),
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
    bench_record_insert(1, 1),
    bench_record_insert(10, 1),
    bench_record_insert(100, 1),
    bench_record_insert(1000, 1),
    bench_record_insert(1, 10),
    bench_record_insert(10, 10),
    bench_record_insert(100, 10),
    bench_record_insert(1000, 10),
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
    bench_table_insert_row(1, 1),
    bench_table_insert_row(10, 1),
    bench_table_insert_row(100, 1),
    bench_table_insert_row(1000, 1),
    bench_table_insert_row(1, 10),
    bench_table_insert_row(10, 10),
    bench_table_insert_row(100, 10),
    bench_table_insert_row(1000, 10),
    bench_table_insert_col(1, 1),
    bench_table_insert_col(10, 1),
    bench_table_insert_col(100, 1),
    bench_table_insert_col(1000, 1),
    bench_table_insert_col(1, 10),
    bench_table_insert_col(10, 10),
    bench_table_insert_col(100, 10),
    bench_table_insert_col(1000, 10),
    // Eval
    // Interleave
    bench_eval_interleave(100),
    bench_eval_interleave(1_000),
    bench_eval_interleave(10_000),
    bench_eval_interleave_with_interrupt(100),
    bench_eval_interleave_with_interrupt(1_000),
    bench_eval_interleave_with_interrupt(10_000),
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
