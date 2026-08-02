#![allow(clippy::unwrap_used)]

use nu_cli::{eval_source, evaluate_commands};
use nu_config::ConfigFileKind;
use nu_experimental::DC_GLOB;
use nu_parser::{lex, lite_parse, parse, parse_block};
use nu_plugin_core::{Encoder, EncodingType};
use nu_plugin_protocol::{PluginCallResponse, PluginOutput};
use nu_protocol::{
    PipelineData, Signals, Span, Spanned, Type, TypeSet, Value,
    ast::PathMember,
    casing::Casing,
    engine::{EngineState, Stack, StateWorkingSet},
};
use nu_std::load_standard_library;
use nu_table::{NuTable, TableTheme};
use std::{
    env,
    fmt::Write,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, OnceLock, atomic::AtomicBool},
};
use tango_bench::{IntoBenchmarks, benchmark_fn, tango_benchmarks, tango_main};
use tempfile::{Builder as TempDirBuilder, TempDir};

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
            engine.signals().reset();
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

struct ExperimentalOptionGuard {
    previous: bool,
}

impl ExperimentalOptionGuard {
    fn set_dc_glob(value: bool) -> Self {
        let previous = DC_GLOB.get();
        // SAFETY: Benchmarks run in a controlled process and restore the previous value on drop.
        unsafe { DC_GLOB.set(value) };
        Self { previous }
    }
}

impl Drop for ExperimentalOptionGuard {
    fn drop(&mut self) {
        // SAFETY: Restores the previous process-global benchmark state.
        unsafe { DC_GLOB.set(self.previous) };
    }
}

struct GlobBenchFixture {
    _tempdir: TempDir,
    root: PathBuf,
}

impl GlobBenchFixture {
    fn from_repo_snapshot(prefix: &str, source_root: &Path) -> Self {
        let tempdir = TempDirBuilder::new().prefix(prefix).tempdir().unwrap();
        copy_repo_snapshot(source_root, tempdir.path());

        Self {
            root: tempdir.path().to_path_buf(),
            _tempdir: tempdir,
        }
    }
}

fn should_skip_repo_entry(name: &str) -> bool {
    matches!(name, ".git" | "target")
}

fn copy_repo_snapshot(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();

    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if should_skip_repo_entry(&file_name) {
            continue;
        }

        let target = destination.join(file_name.as_ref());
        let metadata = fs::symlink_metadata(&path).unwrap();
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            copy_repo_snapshot(&path, &target);
        } else if file_type.is_file() {
            fs::copy(&path, &target).unwrap();
        }
    }
}

fn nu_string_literal(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy())
}

fn setup_stack_and_engine_in_dir(path: &Path) -> (Stack, EngineState) {
    setup_stack_and_engine_from_command(&format!("cd {}", nu_string_literal(path)))
}

// Running benchmarks like this allow you to set the glob/ls benchmarks root folder
// NU_GLOB_BENCH_ROOT=/Users/fdncred/src/nushell cargo bench --bench benchmarks -- solo --filter '*recursive*'
fn glob_bench_source_root() -> PathBuf {
    env::var_os("NU_GLOB_BENCH_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap())
}

fn shared_glob_bench_fixture() -> Arc<GlobBenchFixture> {
    static FIXTURE: OnceLock<Arc<GlobBenchFixture>> = OnceLock::new();

    FIXTURE
        .get_or_init(|| {
            let source_root = glob_bench_source_root();
            Arc::new(GlobBenchFixture::from_repo_snapshot(
                "nu_glob_repo_snapshot",
                &source_root,
            ))
        })
        .clone()
}

fn bench_command_with_dc_glob(
    name: impl Into<String>,
    command: impl Into<String> + Clone,
    dc_glob_enabled: bool,
    stack: Stack,
    engine: EngineState,
    fixture: Arc<GlobBenchFixture>,
) -> impl IntoBenchmarks {
    let commands = Spanned {
        span: Span::test_data(),
        item: command.into(),
    };

    [benchmark_fn(name, move |b| {
        let commands = commands.clone();
        let stack = stack.clone();
        let engine = engine.clone();
        let _fixture = &fixture;

        b.iter(move || {
            let _dc_glob_guard = ExperimentalOptionGuard::set_dc_glob(dc_glob_enabled);
            let mut stack = stack.clone();
            let mut engine = engine.clone();
            engine.signals().reset();

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

fn bench_recursive_glob_command(
    name: &str,
    command: &str,
    dc_glob_enabled: bool,
) -> impl IntoBenchmarks {
    let fixture = shared_glob_bench_fixture();
    let (stack, engine) = setup_stack_and_engine_in_dir(&fixture.root);

    bench_command_with_dc_glob(
        name,
        format!("{command} | length | ignore"),
        dc_glob_enabled,
        stack,
        engine,
        fixture,
    )
}

fn ls_recursive_pattern(root: &Path) -> String {
    let mut root = root.to_string_lossy().replace('\\', "/");
    if root.ends_with('/') {
        root.pop();
    }
    format!("{root}/**/*")
}

fn bench_ls_recursive_command(name: &str, dc_glob_enabled: bool) -> impl IntoBenchmarks {
    let fixture = shared_glob_bench_fixture();
    let (stack, engine) = setup_stack_and_engine_in_dir(&fixture.root);
    let command = format!(
        "ls {} | length | ignore",
        ls_recursive_pattern(&fixture.root)
    );

    bench_command_with_dc_glob(name, command, dc_glob_enabled, stack, engine, fixture)
}

fn bench_ls_recursive_legacy() -> impl IntoBenchmarks {
    bench_ls_recursive_command("ls_recursive_legacy", false)
}

fn bench_ls_recursive_dc() -> impl IntoBenchmarks {
    bench_ls_recursive_command("ls_recursive_dc", true)
}

fn bench_glob_recursive_legacy_wax() -> impl IntoBenchmarks {
    bench_recursive_glob_command("glob_recursive_legacy_wax", "glob '**/*'", false)
}

fn bench_glob_recursive_dc_glob() -> impl IntoBenchmarks {
    bench_recursive_glob_command("glob_recursive_dc_glob", "glob '**/*'", true)
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

fn bench_binary_value_clone(bytes: usize) -> impl IntoBenchmarks {
    let name = format!("binary_value_clone_{bytes}b");
    [benchmark_fn(name, move |b| {
        let value = Value::test_binary(vec![0; bytes]);
        b.iter(move || {
            black_box(value.clone());
        })
    })]
}

fn bench_list_value_clone(items: usize) -> impl IntoBenchmarks {
    let name = format!("list_value_clone_{items}");
    [benchmark_fn(name, move |b| {
        let value = Value::test_list((0..items).map(|i| Value::test_int(i as i64)).collect());
        b.iter(move || {
            black_box(value.clone());
        })
    })]
}

fn bench_stack_list_get_var(items: usize) -> impl IntoBenchmarks {
    use nu_protocol::VarId;

    let name = format!("stack_list_get_var_{items}");
    [benchmark_fn(name, move |b| {
        let mut stack = Stack::new();
        let var_id = VarId::new(0);
        stack.add_var(
            var_id,
            Value::test_list((0..items).map(|i| Value::test_int(i as i64)).collect()),
        );
        b.iter(move || {
            black_box(
                stack
                    .get_var(var_id, Span::test_data())
                    .expect("var present"),
            );
        })
    })]
}

fn bench_binary_slice_into_int(bytes: usize, reads: usize) -> impl IntoBenchmarks {
    let (mut stack, engine) = setup_stack_and_engine_from_command(
        "def bench-u16 [data: binary offset: int] {
    $data | bytes at $offset..<($offset + 2) | into int --endian big
}
let binary = 0x[]",
    );
    let binary_id = StateWorkingSet::new(&engine)
        .find_variable(b"binary")
        .expect("must exist");
    stack.add_var(binary_id, Value::test_binary(vec![0; bytes]));

    bench_command(
        format!("binary_slice_into_int_{bytes}b_{reads}_reads"),
        format!("0..<{reads} | each {{|offset| bench-u16 $binary $offset }} | math sum | ignore"),
        stack,
        engine,
    )
}

fn bench_binary_skip_shared_half(bytes: usize, reads: usize) -> impl IntoBenchmarks {
    let (mut stack, engine) = setup_stack_and_engine_from_command("let binary = 0x[]");
    let binary_id = StateWorkingSet::new(&engine)
        .find_variable(b"binary")
        .expect("must exist");
    // Keep the original in the stack so each `$binary` lookup exercises shared backing storage.
    stack.add_var(binary_id, Value::test_binary(vec![0; bytes]));

    bench_command(
        format!("binary_skip_shared_{bytes}b_half_{reads}_reads"),
        format!(
            "0..<{reads} | each {{ $binary | skip {} | ignore }} | ignore",
            bytes / 2
        ),
        stack,
        engine,
    )
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

fn setup_str_replace_strings(n: usize) -> (Stack, EngineState) {
    setup_stack_and_engine_from_command(&format!(
        r#"let strings = 0..<{n} | each {{ |i| $"abc($i) xyz 123" }}"#
    ))
}

fn setup_str_replace_multiline_strings(n: usize) -> (Stack, EngineState) {
    setup_stack_and_engine_from_command(&format!(
        r#"let strings = 0..<{n} | each {{ |i| $"($i). first\n($i). second\nplain" }}"#
    ))
}

fn setup_str_replace_table(n: usize) -> (Stack, EngineState) {
    setup_stack_and_engine_from_command(&format!(
        r#"let table = 0..<{n} | each {{ |i| {{ a: $"abc($i)", b: $"def($i)", c: untouched }} }}"#
    ))
}

fn bench_str_replace_regex_list(n: usize) -> impl IntoBenchmarks {
    let (stack, engine) = setup_str_replace_strings(n);
    bench_command(
        format!("str_replace_regex_list_{n}"),
        r#"$strings | str replace -a -r '\d+' 'N' | ignore"#,
        stack,
        engine,
    )
}

fn bench_str_replace_regex_table(n: usize) -> impl IntoBenchmarks {
    let (stack, engine) = setup_str_replace_table(n);
    bench_command(
        format!("str_replace_regex_table_{n}_2cols"),
        r#"$table | str replace -a -r '\d+' 'N' a b | ignore"#,
        stack,
        engine,
    )
}

fn bench_str_replace_multiline_list(n: usize) -> impl IntoBenchmarks {
    let (stack, engine) = setup_str_replace_multiline_strings(n);
    bench_command(
        format!("str_replace_multiline_list_{n}"),
        r#"$strings | str replace -a --multiline '^[0-9]+\. ' '' | ignore"#,
        stack,
        engine,
    )
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

/// End-to-end mut field assign via IR `UpdateVarCellPath`.
/// Setup builds a large record once; the timed body only reassigns the field
/// many times so assign cost dominates engine/stack clone overhead.
fn bench_mut_record_assign(n: usize) -> impl IntoBenchmarks {
    let setup = format!("mut r = {{a: (1..{n} | each {{|i| $i | into string}} | str join)}}");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("mut_record_assign_{n}"),
        "for _ in 1..1000 { $r.a = 'x' }",
        stack,
        engine,
    )
}

/// End-to-end mut list element assign (same IR path as record fields).
fn bench_mut_list_assign(n: usize) -> impl IntoBenchmarks {
    let setup = format!("mut l = (1..{n} | each {{|i| {{a: $i}}}})");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("mut_list_assign_{n}"),
        "for _ in 1..1000 { $l.0 = {a: 999} }",
        stack,
        engine,
    )
}

/// Compound field assign (`+=`) still ends in in-place update; payload is large
/// so a clone-on-write regression would show up even though only `a` changes.
fn bench_mut_record_compound_assign(n: usize) -> impl IntoBenchmarks {
    let setup = format!("mut r = {{a: 0, b: (1..{n} | each {{|i| $i | into string}} | str join)}}");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("mut_record_compound_assign_{n}"),
        "for _ in 1..1000 { $r.a += 1 }",
        stack,
        engine,
    )
}

/// Slow baseline: full value replace via pipeline `update` (not the optimized path).
/// Compare against `mut_record_assign_*` on the same `n` to quantify the win.
fn bench_mut_record_update(n: usize) -> impl IntoBenchmarks {
    let setup = format!("mut r = {{a: (1..{n} | each {{|i| $i | into string}} | str join)}}");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("mut_record_update_{n}"),
        "for _ in 1..1000 { $r = ($r | update a { 'x' }) }",
        stack,
        engine,
    )
}

/// Stack-level: old path = clone value + upsert + add_var (forces SharedCow deep-copy).
fn bench_stack_upsert_clone(n: usize) -> impl IntoBenchmarks {
    use nu_protocol::VarId;
    let long_string = "x".repeat(n);
    let cell_path = vec![PathMember::test_string("a", false, Casing::Sensitive)];
    [benchmark_fn(format!("stack_upsert_clone_{n}"), move |b| {
        let mut stack = Stack::new();
        let var_id = VarId::new(0);
        stack.add_var(
            var_id,
            Value::test_record(nu_protocol::record!("a" => Value::test_string(&long_string))),
        );
        let new_val = Value::test_string("x");
        let cell_path = cell_path.clone();
        b.iter(move || {
            // Simulate pre-optimization: lookup clones, mutate copy, write back.
            let mut value = stack
                .get_var(var_id, Span::test_data())
                .expect("var present");
            value
                .upsert_data_at_cell_path(&cell_path, new_val.clone())
                .expect("upsert");
            stack.add_var(var_id, value);
            black_box(&stack);
        })
    })]
}

/// Stack-level: new path = get_var_mut / upsert_var_cell_path (unique ownership).
fn bench_stack_upsert_inplace(n: usize) -> impl IntoBenchmarks {
    use nu_protocol::VarId;
    let long_string = "x".repeat(n);
    let cell_path = vec![PathMember::test_string("a", false, Casing::Sensitive)];
    [benchmark_fn(
        format!("stack_upsert_inplace_{n}"),
        move |b| {
            let mut stack = Stack::new();
            let var_id = VarId::new(0);
            stack.add_var(
                var_id,
                Value::test_record(nu_protocol::record!("a" => Value::test_string(&long_string))),
            );
            let new_val = Value::test_string("x");
            let cell_path = cell_path.clone();
            b.iter(move || {
                stack
                    .upsert_var_cell_path(var_id, &cell_path, new_val.clone(), Span::test_data())
                    .expect("upsert");
                black_box(&stack);
            })
        },
    )]
}

/// Benchmark: clone a shared Record then upsert (simulates old `lookup_var` + upsert path).
/// `original` is retained, so each `clone()` bumps SharedCow refcount and forces a deep-copy
/// on `to_mut()` during upsert.
fn bench_upsert_record_clone(n: usize) -> impl IntoBenchmarks {
    let long_string = "x".repeat(n);
    let original =
        Value::test_record(nu_protocol::record!("a" => Value::test_string(&long_string)));
    [benchmark_fn(format!("upsert_record_clone_{n}"), move |b| {
        let original = original.clone();
        let new_val = Value::test_string("x");
        let cell_path = vec![PathMember::test_string("a", false, Casing::Sensitive)];
        b.iter(move || {
            let mut cloned = original.clone();
            cloned
                .upsert_data_at_cell_path(&cell_path, new_val.clone())
                .unwrap();
            black_box(cloned);
        })
    })]
}

/// Benchmark: upsert a uniquely-owned Record in place (simulates `get_var_mut` when the
/// stack value is not shared). Setup builds one value; timed loop only mutates it.
fn bench_upsert_record_inplace(n: usize) -> impl IntoBenchmarks {
    let long_string = "x".repeat(n);
    [benchmark_fn(
        format!("upsert_record_inplace_{n}"),
        move |b| {
            let mut value =
                Value::test_record(nu_protocol::record!("a" => Value::test_string(&long_string)));
            let new_val = Value::test_string("x");
            let cell_path = vec![PathMember::test_string("a", false, Casing::Sensitive)];
            b.iter(move || {
                value
                    .upsert_data_at_cell_path(&cell_path, new_val.clone())
                    .unwrap();
                black_box(&value);
            })
        },
    )]
}

/// Benchmark: clone a shared List then upsert (simulates old path).
fn bench_upsert_list_clone(n: usize) -> impl IntoBenchmarks {
    let original = Value::test_list(
        (0..n)
            .map(|i| Value::test_record(nu_protocol::record!("a" => Value::test_int(i as i64))))
            .collect(),
    );
    [benchmark_fn(format!("upsert_list_clone_{n}"), move |b| {
        let original = original.clone();
        let new_val = Value::test_record(nu_protocol::record!("a" => Value::test_int(999)));
        let cell_path = vec![PathMember::test_int(0, false)];
        b.iter(move || {
            let mut cloned = original.clone();
            cloned
                .upsert_data_at_cell_path(&cell_path, new_val.clone())
                .unwrap();
            black_box(cloned);
        })
    })]
}

/// Benchmark: upsert a uniquely-owned List in place (no shared clone). Construction is
/// outside the timed loop so this isolates mutation cost vs `upsert_list_clone_*`.
fn bench_upsert_list_inplace(n: usize) -> impl IntoBenchmarks {
    [benchmark_fn(format!("upsert_list_inplace_{n}"), move |b| {
        let mut value = Value::test_list(
            (0..n)
                .map(|i| Value::test_record(nu_protocol::record!("a" => Value::test_int(i as i64))))
                .collect(),
        );
        let new_val = Value::test_record(nu_protocol::record!("a" => Value::test_int(999)));
        let cell_path = vec![PathMember::test_int(0, false)];
        b.iter(move || {
            value
                .upsert_data_at_cell_path(&cell_path, new_val.clone())
                .unwrap();
            black_box(&value);
        })
    })]
}

/// Pure `Value::concat` (both sides non-empty). Isolates operator cost from pipeline setup.
fn bench_value_concat_general(n: usize) -> impl IntoBenchmarks {
    let lhs = Value::test_list((0..n).map(|i| Value::test_int(i as i64)).collect());
    let rhs = Value::test_list((0..n).map(|i| Value::test_int(i as i64)).collect());
    [benchmark_fn(
        format!("value_concat_general_{n}"),
        move |b| {
            let lhs = lhs.clone();
            let rhs = rhs.clone();
            b.iter(move || {
                black_box(
                    lhs.concat(Span::test_data(), &rhs, Span::test_data())
                        .expect("concat"),
                );
            })
        },
    )]
}

/// Pure `Value::concat` empty-LHS shortcut (`[] ++ xs`).
fn bench_value_concat_empty_lhs(n: usize) -> impl IntoBenchmarks {
    let empty = Value::test_list(vec![]);
    let rhs = Value::test_list((0..n).map(|i| Value::test_int(i as i64)).collect());
    [benchmark_fn(
        format!("value_concat_empty_lhs_{n}"),
        move |b| {
            let empty = empty.clone();
            let rhs = rhs.clone();
            b.iter(move || {
                black_box(
                    empty
                        .concat(Span::test_data(), &rhs, Span::test_data())
                        .expect("concat"),
                );
            })
        },
    )]
}

/// Pure `Value::concat` empty-RHS shortcut (`xs ++ []`).
fn bench_value_concat_empty_rhs(n: usize) -> impl IntoBenchmarks {
    let lhs = Value::test_list((0..n).map(|i| Value::test_int(i as i64)).collect());
    let empty = Value::test_list(vec![]);
    [benchmark_fn(
        format!("value_concat_empty_rhs_{n}"),
        move |b| {
            let lhs = lhs.clone();
            let empty = empty.clone();
            b.iter(move || {
                black_box(
                    lhs.concat(Span::test_data(), &empty, Span::test_data())
                        .expect("concat"),
                );
            })
        },
    )]
}

/// Command-level concat with **prebuilt** lists on the stack (no `1..n | each` in the timed path).
fn bench_concat_prebuilt_general(n: usize) -> impl IntoBenchmarks {
    let setup = format!("let a = 1..{n} | each {{|i| $i}}; let b = 1..{n} | each {{|i| $i}}");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("concat_prebuilt_general_{n}"),
        // Amplify: 100 concats per iter so operator work exceeds engine clone noise.
        "for _ in 1..100 { $a ++ $b | ignore }",
        stack,
        engine,
    )
}

fn bench_concat_prebuilt_empty_lhs(n: usize) -> impl IntoBenchmarks {
    let setup = format!("let a = []; let b = 1..{n} | each {{|i| $i}}");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("concat_prebuilt_empty_lhs_{n}"),
        "for _ in 1..100 { $a ++ $b | ignore }",
        stack,
        engine,
    )
}

fn bench_concat_prebuilt_empty_rhs(n: usize) -> impl IntoBenchmarks {
    let setup = format!("let a = 1..{n} | each {{|i| $i}}; let b = []");
    let (stack, engine) = setup_stack_and_engine_from_command(&setup);
    bench_command(
        format!("concat_prebuilt_empty_rhs_{n}"),
        "for _ in 1..100 { $a ++ $b | ignore }",
        stack,
        engine,
    )
}

/// Single large `par-each` without `--threads` (global pool; work dominates for large n).
fn bench_par_each_default_pool(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    bench_command(
        format!("par_each_default_pool_{n}"),
        format!("(1..{n}) | par-each {{|_| 1 }} | ignore"),
        stack,
        engine,
    )
}

/// Many sequential small `par-each` calls: amplifies pool create/reuse.
/// Compare branch vs main — main paid a new private pool per call.
fn bench_par_each_many_calls(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    let cmds = (0..n)
        .map(|_| "(1..10) | par-each {|_| 1 } | ignore")
        .collect::<Vec<_>>()
        .join("; ");
    bench_command(format!("par_each_many_calls_{n}"), cmds, stack, engine)
}

/// Same as many_calls but with an explicit `-t 2` (cached custom pool path).
fn bench_par_each_many_calls_threads(n: usize) -> impl IntoBenchmarks {
    let engine = setup_engine();
    let stack = Stack::new();
    let cmds = (0..n)
        .map(|_| "(1..10) | par-each -t 2 {|_| 1 } | ignore")
        .collect::<Vec<_>>()
        .join("; ");
    bench_command(
        format!("par_each_many_calls_threads_{n}"),
        cmds,
        stack,
        engine,
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
        bench.iter(move || black_box(a.clone().union(b.clone())))
    })]
}

fn bench_type_widen_large_records() -> impl IntoBenchmarks {
    let rec1: Type = Type::Record((0..50).map(|i| (format!("f{i}"), Type::Int)).collect());
    let rec2: Type = Type::Record((0..50).map(|i| (format!("f{i}"), Type::Number)).collect());
    [benchmark_fn("type_widen_large_records", move |bench| {
        let rec1 = rec1.clone();
        let rec2 = rec2.clone();
        bench.iter(move || black_box(rec1.clone().union(rec2.clone())))
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
        bench.iter(move || black_box(one.clone().union(two.clone())))
    })]
}

fn bench_type_widen_chain() -> impl IntoBenchmarks {
    let mut t = Type::String;
    for _ in 0..100 {
        t = t.union(Type::Int);
    }
    [benchmark_fn("type_widen_chain", move |bench| {
        let t = t.clone();
        bench.iter(move || {
            let mut tmp = t.clone();
            tmp = tmp.union(Type::Int);
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
            black_box(parse_block(
                &mut working_set,
                &tokens,
                span,
                true,
                false,
                None,
            ));
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
    bench_ls_recursive_legacy(),
    bench_ls_recursive_dc(),
    bench_glob_recursive_legacy_wax(),
    bench_glob_recursive_dc_glob(),
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
    // Binary
    bench_binary_value_clone(2 * 1024 * 1024),
    bench_binary_slice_into_int(2 * 1024 * 1024, 100),
    bench_binary_skip_shared_half(2 * 1024 * 1024, 100),
    // List
    bench_list_value_clone(100_000),
    bench_stack_list_get_var(100_000),
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
    // Strings
    bench_str_replace_regex_list(1_000),
    bench_str_replace_regex_table(500),
    bench_str_replace_multiline_list(1_000),
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
    // Par-Each: default global pool (work-dominated for large n)
    bench_par_each_default_pool(100),
    bench_par_each_default_pool(1_000),
    bench_par_each_default_pool(10_000),
    // Par-Each: many sequential small calls (pool create/reuse — strongest par-each signal)
    bench_par_each_many_calls(10),
    bench_par_each_many_calls(50),
    bench_par_each_many_calls(100),
    bench_par_each_many_calls_threads(10),
    bench_par_each_many_calls_threads(50),
    bench_par_each_many_calls_threads(100),
    // Config
    bench_eval_default_config(),
    // Env
    bench_eval_default_env(),
    // Mut field assign (IR UpdateVarCellPath) — compare vs mut_record_update_* baseline
    bench_mut_record_assign(1_000),
    bench_mut_record_assign(10_000),
    bench_mut_record_assign(100_000),
    bench_mut_list_assign(1_000),
    bench_mut_list_assign(10_000),
    bench_mut_list_assign(100_000),
    bench_mut_record_compound_assign(1_000),
    bench_mut_record_compound_assign(10_000),
    bench_mut_record_compound_assign(100_000),
    bench_mut_record_update(1_000),
    bench_mut_record_update(10_000),
    bench_mut_record_update(100_000),
    // Stack-level mut path: clone+add_var vs upsert_var_cell_path
    bench_stack_upsert_clone(1_000),
    bench_stack_upsert_clone(10_000),
    bench_stack_upsert_clone(100_000),
    bench_stack_upsert_inplace(1_000),
    bench_stack_upsert_inplace(10_000),
    bench_stack_upsert_inplace(100_000),
    // Pure Value::concat (no engine noise)
    bench_value_concat_general(1_000),
    bench_value_concat_general(10_000),
    bench_value_concat_general(100_000),
    bench_value_concat_empty_lhs(1_000),
    bench_value_concat_empty_lhs(10_000),
    bench_value_concat_empty_lhs(100_000),
    bench_value_concat_empty_rhs(1_000),
    bench_value_concat_empty_rhs(10_000),
    bench_value_concat_empty_rhs(100_000),
    // Command-level concat with prebuilt stack vars
    bench_concat_prebuilt_general(1_000),
    bench_concat_prebuilt_general(10_000),
    bench_concat_prebuilt_general(100_000),
    bench_concat_prebuilt_empty_lhs(1_000),
    bench_concat_prebuilt_empty_lhs(10_000),
    bench_concat_prebuilt_empty_lhs(100_000),
    bench_concat_prebuilt_empty_rhs(1_000),
    bench_concat_prebuilt_empty_rhs(10_000),
    bench_concat_prebuilt_empty_rhs(100_000),
    // Raw Value upsert: clone path vs unique-ownership path
    bench_upsert_record_clone(1_000),
    bench_upsert_record_clone(10_000),
    bench_upsert_record_clone(100_000),
    bench_upsert_record_inplace(1_000),
    bench_upsert_record_inplace(10_000),
    bench_upsert_record_inplace(100_000),
    bench_upsert_list_clone(1_000),
    bench_upsert_list_clone(10_000),
    bench_upsert_list_clone(100_000),
    bench_upsert_list_inplace(1_000),
    bench_upsert_list_inplace(10_000),
    bench_upsert_list_inplace(100_000),
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
