#![allow(clippy::unwrap_used)]

use nu_cli::{eval_source, evaluate_commands};
use nu_parser::{lex, lite_parse, parse, parse_block};
use nu_plugin_core::{Encoder, EncodingType};
use nu_plugin_protocol::{PluginCallResponse, PluginOutput};
use nu_protocol::{
    PipelineData, Signals, Span, Spanned, Type, Value,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_std::load_standard_library;
use nu_table::{NuTable, TableTheme};
use nu_utils::ConfigFileKind;
use std::{
    fmt::Write,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
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
        span: Span::test_data(),
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
        span: Span::test_data(),
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
            span: Span::test_data(),
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
        write!(s, "[0, 1, {i}]").expect("writing to a String is infallible");
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

// Benchmarks specifically for Type widening logic.

fn bench_type_widen_simple() -> impl IntoBenchmarks {
    let a = Type::Int;
    let b = Type::Float;
    [benchmark_fn("type_widen_simple", move |bench| {
        let a = a.clone();
        let b = b.clone();
        bench.iter(move || black_box(a.clone().widen(b.clone())))
    })]
}

fn bench_type_widen_large_records() -> impl IntoBenchmarks {
    let rec1: Type = Type::Record(
        (0..50)
            .map(|i| (format!("f{i}"), Type::Int))
            .collect::<Vec<_>>()
            .into(),
    );
    let rec2: Type = Type::Record(
        (0..50)
            .map(|i| (format!("f{i}"), Type::Number))
            .collect::<Vec<_>>()
            .into(),
    );
    [benchmark_fn("type_widen_large_records", move |bench| {
        let rec1 = rec1.clone();
        let rec2 = rec2.clone();
        bench.iter(move || black_box(rec1.clone().widen(rec2.clone())))
    })]
}

fn bench_type_widen_large_oneof() -> impl IntoBenchmarks {
    let one: Type = Type::one_of(
        (0..32)
            .map(|i| Type::Record(vec![(format!("f{i}"), Type::Int)].into()))
            .collect::<Vec<_>>(),
    );
    let two: Type = Type::one_of(
        (0..32)
            .map(|i| Type::Record(vec![(format!("f{i}"), Type::Number)].into()))
            .collect::<Vec<_>>(),
    );
    [benchmark_fn("type_widen_large_oneof", move |bench| {
        let one = one.clone();
        let two = two.clone();
        bench.iter(move || black_box(one.clone().widen(two.clone())))
    })]
}

fn bench_type_widen_chain() -> impl IntoBenchmarks {
    let mut t = Type::String;
    for _ in 0..100 {
        t = t.widen(Type::Int);
    }
    [benchmark_fn("type_widen_chain", move |bench| {
        let t = t.clone();
        bench.iter(move || {
            let mut tmp = t.clone();
            tmp = tmp.widen(Type::Int);
            black_box(tmp)
        })
    })]
}

// Parsing benchmarks (nu-parser)
// These benchmark names intentionally include source byte/char sizes for throughput reporting.
// Benchmarks are broken into stages (lex, lite, parse_block, full parse) with three corpus sizes:
// - small: synthetic short pipeline (noise-resistant signal)
// - medium: synthetic generated pipeline (scales predictably)
// - large: entire nu-std/std directory (all real stdlib code)
// - real_world: toolkit/mod.nu (representative Nushell scripts)
//
// Each benchmark name encodes byte/char counts: parser_<stage>_<dataset>_<size>b_<chars>c
// This format enables automatic derivation of throughput metrics (chars/sec, bytes/sec)
// for publication in PR descriptions and performance tracking.

const PARSER_REAL_WORLD_TOOLKIT_MOD: &str = include_str!("../toolkit/mod.nu");

/// Recursively discover all .nu files under a directory.
fn collect_nu_files_recursive(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_nu_files_recursive(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "nu") {
            files.push(path);
        }
    }
}

/// Collect all .nu source files from crates/nu-std/std into a single concatenated string.
/// Files are sorted by path for deterministic output across runs.
/// Returns None if the directory cannot be read (will panic at benchmark time to fail loudly).
fn collect_all_std_nu_sources() -> Option<String> {
    let std_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/nu-std/std");
    let mut files = Vec::new();
    collect_nu_files_recursive(&std_root, &mut files);

    files.sort();

    let mut combined = String::new();
    for path in files {
        if let Ok(contents) = fs::read_to_string(path) {
            combined.push_str(&contents);
            combined.push('\n');
        }
    }

    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

fn parser_bench_name(stage: &str, dataset: &str, source: &str) -> String {
    // Encode the byte and character counts in the benchmark name for automatic throughput derivation.
    // Example: parser_lex_small_127b_125c
    format!(
        "parser_{stage}_{dataset}_{}b_{}c",
        source.len(),
        source.chars().count()
    )
}

/// Generate a synthetic medium-sized Nushell script with multiple pipelines.
/// Scales predictably with `commands` parameter for A/B testing.
fn create_parser_pipeline_script(commands: usize) -> String {
    let mut script = String::new();

    for i in 0..commands {
        writeln!(
            script,
            "let row_{i} = ({i}..{} | each {{|x| $x + 1 }} | math sum)",
            i + 20
        )
        .expect("writing to a String is infallible");
    }

    script.push_str("$row_0 | ignore");
    script
}

/// Set up a minimal EngineState for parser benchmarks (required for StateWorkingSet).
/// Sets PWD environment variable to allow parsing code that references it.
fn parser_engine_state() -> EngineState {
    let mut engine_state = EngineState::new();

    if let Ok(cwd) = std::env::current_dir() {
        engine_state.add_env_var(
            "PWD".into(),
            Value::string(cwd.to_string_lossy().to_string(), Span::test_data()),
        );
    }

    engine_state
}

/// Benchmark lexer throughput: tokenize input without parsing or AST construction.
/// Isolates lexer performance from parser/AST overhead.
/// Benchmark name format: parser_lex_<dataset>_<size>b_<chars>c
fn bench_parser_lex(dataset: &str, source: String) -> impl IntoBenchmarks {
    let bench_name = parser_bench_name("lex", dataset, &source);
    let input = source.into_bytes();

    [benchmark_fn(bench_name, move |b| {
        let input = input.clone();
        b.iter(move || {
            black_box(lex(&input, 0, &[], &[], false));
        })
    })]
}

/// Benchmark lite-parse stage: convert tokens to LiteBlock (pipeline structure, no AST).
/// Isolates the pipeline/command grouping logic from full AST generation.
/// Benchmark name format: parser_lite_<dataset>_<size>b_<chars>c
fn bench_parser_lite(dataset: &str, source: String) -> impl IntoBenchmarks {
    let bench_name = parser_bench_name("lite", dataset, &source);
    let input = source.into_bytes();

    [benchmark_fn(bench_name, move |b| {
        let input = input.clone();
        b.iter(move || {
            let (tokens, lex_error) = lex(&input, 0, &[], &[], false);
            assert!(
                lex_error.is_none(),
                "parser benchmark input must lex cleanly"
            );
            let engine_state = parser_engine_state();
            let working_set = StateWorkingSet::new(&engine_state);
            black_box(lite_parse(&tokens, &working_set));
        })
    })]
}

/// Benchmark parse_block stage: convert pre-lexed tokens to Block (AST without compilation).
/// Isolates AST generation cost from lexing overhead.
/// Each iteration re-lexes to keep ownership model clean.
/// Benchmark name format: parser_parse_block_<dataset>_<size>b_<chars>c
fn bench_parser_parse_block(dataset: &str, source: String) -> impl IntoBenchmarks {
    let bench_name = parser_bench_name("parse_block", dataset, &source);
    let input = source.into_bytes();

    [benchmark_fn(bench_name, move |b| {
        let input = input.clone();
        b.iter(move || {
            let (tokens, lex_error) = lex(&input, 0, &[], &[], false);
            assert!(
                lex_error.is_none(),
                "parser benchmark input must lex cleanly"
            );
            let span = Span::new(0, input.len());
            let engine_state = parser_engine_state();
            let mut working_set = StateWorkingSet::new(&engine_state);
            black_box(parse_block(&mut working_set, &tokens, span, true, false));
        })
    })]
}

/// Benchmark full parse pipeline: lex + lite-parse + AST + compilation.
/// End-to-end measurement from source bytes to compiled Block.
/// Includes eager IR compilation of top-level blocks.
/// Benchmark name format: parser_parse_<dataset>_<size>b_<chars>c
fn bench_parser_full_parse(dataset: &str, source: String) -> impl IntoBenchmarks {
    let bench_name = parser_bench_name("parse", dataset, &source);
    let input = source.into_bytes();

    [benchmark_fn(bench_name, move |b| {
        let input = input.clone();
        b.iter(move || {
            let engine_state = parser_engine_state();
            let mut working_set = StateWorkingSet::new(&engine_state);
            black_box(parse(
                &mut working_set,
                Some("parser_bench.nu"),
                &input,
                true,
            ));
        })
    })]
}

/// Small parser benchmark input: synthetic short pipeline (~127 bytes).
/// Measures lex/parse overhead with minimal variability for signal clarity.
fn parser_input_small() -> String {
    "let xs = [1 2 3 4 5]\n$xs | each {|x| $x + 1 } | where $it > 2 | math sum | ignore\n"
        .to_string()
}

/// Medium parser benchmark input: synthetic 80-command pipeline (scales predictably).
/// Measures throughput on moderately-sized real-world scripts.
fn parser_input_medium() -> String {
    create_parser_pipeline_script(80)
}

/// Large parser benchmark input: entire crates/nu-std/std directory as concatenated .nu files.
/// Comprehensive real-world benchmark covering all stdlib modules.
/// Panics if directory discovery or file reads fail to ensure reliable benchmarks.
fn parser_input_large() -> String {
    collect_all_std_nu_sources().expect("parser benchmark requires readable nu-std/std directory")
}

/// Real-world parser benchmark input: toolkit/mod.nu (Nushell dev scripts).
/// Provides realistic parsing workload covering procedural and module code.
fn parser_input_real_world() -> String {
    PARSER_REAL_WORLD_TOOLKIT_MOD.to_string()
}

// Table rendering benchmarks (nu-table)
// Benchmark the NuTable::draw path for varying table sizes and themes.

fn create_nu_table(rows: usize, cols: usize) -> NuTable {
    let mut table = NuTable::new(rows + 1, cols);

    // Header row
    for col in 0..cols {
        table.insert((0, col), format!("column_{col}"));
    }

    // Data rows
    for row in 0..rows {
        for col in 0..cols {
            table.insert((row + 1, col), format!("value_{row}_{col}"));
        }
    }

    table.set_structure(false, true, false);
    table
}

fn bench_table_render(rows: usize, cols: usize) -> impl IntoBenchmarks {
    let name = format!("table_render_{rows}x{cols}");
    [benchmark_fn(name, move |b| {
        let table = create_nu_table(rows, cols);
        b.iter(move || {
            black_box(table.clone().draw(200));
        })
    })]
}

fn bench_table_render_themed(rows: usize, cols: usize) -> impl IntoBenchmarks {
    let name = format!("table_render_{rows}x{cols}_rounded");
    [benchmark_fn(name, move |b| {
        let mut table = create_nu_table(rows, cols);
        table.set_theme(TableTheme::rounded());
        b.iter(move || {
            black_box(table.clone().draw(200));
        })
    })]
}

fn bench_table_render_wide(cols: usize) -> impl IntoBenchmarks {
    let name = format!("table_render_10x{cols}_wide");
    [benchmark_fn(name, move |b| {
        let table = create_nu_table(10, cols);
        b.iter(move || {
            black_box(table.clone().draw(1000));
        })
    })]
}

tango_benchmarks!(
    bench_load_standard_lib(),
    bench_load_use_standard_lib(),
    // type-widening microbenchmarks (run on both branch & main to compare)
    bench_type_widen_simple(),
    bench_type_widen_large_records(),
    bench_type_widen_large_oneof(),
    bench_type_widen_chain(),
    // Parsing (nu-parser)
    bench_parser_lex("small", parser_input_small()),
    bench_parser_lex("medium", parser_input_medium()),
    bench_parser_lex("large", parser_input_large()),
    bench_parser_lite("small", parser_input_small()),
    bench_parser_lite("medium", parser_input_medium()),
    bench_parser_lite("real_world", parser_input_real_world()),
    bench_parser_parse_block("small", parser_input_small()),
    bench_parser_parse_block("medium", parser_input_medium()),
    bench_parser_parse_block("real_world", parser_input_real_world()),
    bench_parser_full_parse("small", parser_input_small()),
    bench_parser_full_parse("medium", parser_input_medium()),
    bench_parser_full_parse("large", parser_input_large()),
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
    decode_msgpack(10000, 15),
    // Table rendering (nu-table)
    bench_table_render(10, 5),
    bench_table_render(100, 5),
    bench_table_render(1_000, 5),
    bench_table_render(100, 10),
    bench_table_render_themed(10, 5),
    bench_table_render_themed(100, 5),
    bench_table_render_themed(1_000, 5),
    bench_table_render_wide(20),
    bench_table_render_wide(50)
);

tango_main!();
