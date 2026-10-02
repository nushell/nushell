//! Compare nu-parser's two front ends, the classic one and `winnow-parser`, inside one process.
//!
//! ```text
//! frontends compare [--show] FILE|DIR ...     # same AST from both front ends? (first difference per file)
//! frontends bench [--iters N] FILE|DIR ...    # parse time of each front end, and what winnow parsed
//! ```
//!
//! Both front ends parse each file in a fresh working set over the same engine (the `nu`
//! binary's commands and the standard library), with the file on the file stack so relative
//! `use` resolves. `compare` renders each result as text with every id resolved to what it
//! names (a variable's name and declaration, a command's name, a block's contents inline), so
//! two front ends that create things in a different order still compare equal when they built
//! the same program. It also compares the declarations each parse added, the parse errors and
//! the syntax highlighting (`flatten_block`).

use std::{
    collections::HashSet,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use nu_parser::{flatten_block, parse, winnow_stats};
use nu_protocol::{
    BlockId, ENV_VARIABLE_ID, IN_VARIABLE_ID, LAST_VARIABLE_ID, NU_VARIABLE_ID, Signature, VarId,
    ast::{
        Argument, Block, Expr, Expression, ExternalArgument, ListItem, MatchPattern, Pattern,
        PipelineRedirection, RecordItem, RedirectionTarget,
    },
    engine::{EngineState, StateWorkingSet},
};
use nushell_harness::engine;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((mode, rest)) = args.split_first() else {
        eprintln!("usage: frontends compare|bench [options] FILE|DIR ...");
        std::process::exit(2);
    };
    let mut iters = 10;
    let mut show = false;
    let mut clean = false;
    let mut paths = Vec::new();
    let mut it = rest.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--iters" => iters = it.next().and_then(|n| n.parse().ok()).unwrap_or(iters),
            "--show" => show = true,
            "--clean" => clean = true,
            path => paths.push(PathBuf::from(path)),
        }
    }
    let mut files = Vec::new();
    for path in &paths {
        collect(path, &mut files);
    }
    let files: Vec<PathBuf> = files
        .into_iter()
        .map(|file| std::fs::canonicalize(&file).unwrap_or(file))
        .collect();
    set_winnow(false);
    let engine_state = engine(true);
    match mode.as_str() {
        "compare" => compare(&engine_state, &files, show),
        "bench" => bench(&engine_state, &files, iters, clean),
        "syntax" => syntax(&engine_state, &files, iters),
        // For a profiler: parse the files over and over with one front end.
        "loop-classic" | "loop-winnow" => {
            let winnow = mode == "loop-winnow";
            let sources: Vec<(PathBuf, Vec<u8>)> = files
                .iter()
                .filter_map(|file| Some((file.clone(), std::fs::read(file).ok()?)))
                .collect();
            for _ in 0..iters {
                for (file, source) in &sources {
                    let (working_set, block) = parse_file(&engine_state, file, source, winnow);
                    std::hint::black_box((working_set.parse_errors.len(), block));
                }
            }
        }
        _ => {
            eprintln!("unknown mode `{mode}`");
            std::process::exit(2);
        }
    }
}

fn set_winnow(on: bool) {
    // SAFETY: the option is only switched between parses, in this single-threaded tool.
    unsafe { nu_experimental::WINNOW_PARSER.set(on) };
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
            .map(|dir| dir.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        entries.sort();
        for entry in entries {
            collect(&entry, out);
        }
    } else if path.extension().is_some_and(|ext| ext == "nu") {
        out.push(path.to_path_buf());
    }
}

/// Parse `source` as the file `path` (canonical), the winnow front end on or off.
fn parse_file<'e>(
    engine_state: &'e EngineState,
    path: &Path,
    source: &[u8],
    winnow: bool,
) -> (StateWorkingSet<'e>, Arc<Block>) {
    set_winnow(winnow);
    let mut working_set = StateWorkingSet::new(engine_state);
    let _ = working_set.files.push(path.to_path_buf(), nu_protocol::Span::unknown());
    let block = parse(&mut working_set, Some(&path.to_string_lossy()), source, false);
    working_set.files.pop();
    set_winnow(false);
    (working_set, block)
}

fn compare(engine_state: &EngineState, files: &[PathBuf], show: bool) {
    let (mut same, mut different) = (0, 0);
    for file in files {
        let Ok(source) = std::fs::read(file) else {
            continue;
        };
        let (classic_ws, classic_block) = parse_file(engine_state, file, &source, false);
        let classic = render(&classic_ws, &classic_block);
        let (winnow_ws, winnow_block) = parse_file(engine_state, file, &source, true);
        let winnow = render(&winnow_ws, &winnow_block);
        if classic == winnow {
            same += 1;
            if show {
                println!("--- classic (same as winnow) {}\n{classic}", file.display());
            }
            continue;
        }
        different += 1;
        let classic_lines: Vec<&str> = classic.lines().collect();
        let winnow_lines: Vec<&str> = winnow.lines().collect();
        let at = classic_lines
            .iter()
            .zip(&winnow_lines)
            .position(|(a, b)| a != b)
            .unwrap_or(classic_lines.len().min(winnow_lines.len()));
        println!("DIFF {} (line {at})", file.display());
        // The source around the first span on the classic side's differing line.
        let base = classic_block.span.map_or(0, |span| span.start);
        if let Some(line) = classic_lines.get(at)
            && let Some(start) = first_span_start(line)
            && start >= base
        {
            let offset = start - base;
            let text = String::from_utf8_lossy(&source);
            let from = text[..offset.min(text.len())].rfind('\n').map_or(0, |i| i + 1);
            let to = text[offset.min(text.len())..].find('\n').map_or(text.len(), |i| offset + i);
            println!("  source  | {}", &text[from..to]);
        }
        let from = at.saturating_sub(3);
        for line in classic_lines.iter().skip(from).take(6) {
            println!("  classic | {line}");
        }
        for line in winnow_lines.iter().skip(from).take(6) {
            println!("  winnow  | {line}");
        }
        if show {
            println!("--- classic\n{classic}\n--- winnow\n{winnow}");
        }
    }
    let stats = winnow_stats();
    println!("{same} files identical, {different} different; {stats:?}");
}

/// The mean time of the winnow parser alone over `source`, statement by statement over the
/// engine's commands, the statements dropped: what the winnow front end costs before lowering.
fn syntax_time(engine_state: &EngineState, source: &str, iters: u32) -> Duration {
    syntax_time_with(engine_state, source, iters, None)
}

/// The first words of the engine's multi-word command names.
fn command_prefixes(engine_state: &EngineState) -> HashSet<String> {
    (0..engine_state.num_decls())
        .filter_map(|id| {
            let name = engine_state.get_decl(nu_protocol::DeclId::new(id)).name();
            name.split_once(' ').map(|(first, _)| first.to_string())
        })
        .collect()
}

fn syntax_time_with(
    engine_state: &EngineState,
    source: &str,
    iters: u32,
    prefixes: Option<&HashSet<String>>,
) -> Duration {
    struct Lookup<'a, 'e>(&'a StateWorkingSet<'e>, Option<&'a HashSet<String>>);
    impl nu_winnow_parser::CommandLookup for Lookup<'_, '_> {
        fn find_decl(&self, name: &str) -> Option<nu_winnow_parser::DeclKind> {
            self.0.find_decl(name.as_bytes()).map(|_| nu_winnow_parser::DeclKind::Declared)
        }
        fn is_decl_name_prefix(&self, word: &str) -> bool {
            self.1.is_none_or(|prefixes| prefixes.contains(word))
        }
        fn is_builtin_decl(&self, _name: &str) -> bool {
            true
        }
    }
    struct Drop;
    impl<'a> nu_winnow_parser::BlockSink<'a> for Drop {
        fn predecl(&mut self, def: nu_winnow_parser::PredeclaredDef<'a>) {
            std::hint::black_box(def);
        }
        fn statement(&mut self, pipeline: nu_winnow_parser::ast::Pipeline<'a>, _: Vec<nu_winnow_parser::Diagnostic>) -> bool {
            std::hint::black_box(pipeline);
            true
        }
    }
    let working_set = StateWorkingSet::new(engine_state);
    let lookup = Lookup(&working_set, prefixes);
    let span = nu_winnow_parser::Span::new(0, source.len());
    let mut rounds: Vec<Duration> = (0..5)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..iters {
                let _ = nu_winnow_parser::parse_block_streaming(source, span, &lookup, &mut Drop);
            }
            start.elapsed() / iters.max(1)
        })
        .collect();
    rounds.sort();
    rounds[2]
}

/// The winnow parser alone, statement by statement over the engine's commands, the statements
/// dropped: what the winnow front end costs before lowering; with every word a possible first
/// word of a longer command name (as the lowering's lookup answers), and with an index of the
/// first words of the engine's multi-word names.
fn syntax(engine_state: &EngineState, files: &[PathBuf], iters: u32) {
    let prefixes = command_prefixes(engine_state);
    let (mut every, mut indexed) = (Duration::ZERO, Duration::ZERO);
    for file in files {
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };
        every += syntax_time_with(engine_state, &source, iters, None);
        indexed += syntax_time_with(engine_state, &source, iters, Some(&prefixes));
    }
    println!("syntax: every word a prefix {every:.2?}, with a prefix index {indexed:.2?}");
}

#[allow(dead_code)]
fn syntax_old(engine_state: &EngineState, files: &[PathBuf], iters: u32) {
    struct Lookup<'a, 'e>(&'a StateWorkingSet<'e>);
    impl nu_winnow_parser::CommandLookup for Lookup<'_, '_> {
        fn find_decl(&self, name: &str) -> Option<nu_winnow_parser::DeclKind> {
            self.0.find_decl(name.as_bytes()).map(|_| nu_winnow_parser::DeclKind::Declared)
        }
        fn is_decl_name_prefix(&self, _word: &str) -> bool {
            true
        }
        fn is_builtin_decl(&self, _name: &str) -> bool {
            true
        }
    }
    struct Drop;
    impl<'a> nu_winnow_parser::BlockSink<'a> for Drop {
        fn predecl(&mut self, def: nu_winnow_parser::PredeclaredDef<'a>) {
            std::hint::black_box(def);
        }
        fn statement(&mut self, pipeline: nu_winnow_parser::ast::Pipeline<'a>, _: Vec<nu_winnow_parser::Diagnostic>) -> bool {
            std::hint::black_box(pipeline);
            true
        }
    }
    let working_set = StateWorkingSet::new(engine_state);
    let lookup = Lookup(&working_set);
    let (mut streaming, mut plain) = (Duration::ZERO, Duration::ZERO);
    for file in files {
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };
        let span = nu_winnow_parser::Span::new(0, source.len());
        let start = Instant::now();
        for _ in 0..iters {
            let _ = nu_winnow_parser::parse_block_streaming(&source, span, &lookup, &mut Drop);
        }
        let one = start.elapsed() / iters;
        let start = Instant::now();
        let config = nu_winnow_parser::ParseConfig::new();
        for _ in 0..iters {
            std::hint::black_box(nu_winnow_parser::parse_lenient(&source, &config));
        }
        let two = start.elapsed() / iters;
        streaming += one;
        plain += two;
        println!("{:<60} streaming {one:>9.1?}  standalone {two:>9.1?}", file.display());
    }
    println!("TOTAL streaming {streaming:.2?}, standalone {plain:.2?}");
}

/// The start of the first `start..end` span in a rendered line.
fn first_span_start(line: &str) -> Option<usize> {
    line.split(|c: char| !c.is_ascii_digit() && c != '.')
        .find_map(|word| word.split_once("..").and_then(|(start, _)| start.parse().ok()))
}

fn bench(engine_state: &EngineState, files: &[PathBuf], iters: u32, clean: bool) {
    // Each file: a warm-up, then `ROUNDS` rounds of `iters` classic parses and `iters` winnow
    // parses, alternating; the median round of each is reported.
    const ROUNDS: usize = 7;
    let (mut classic_total, mut winnow_total, mut syntax_total, mut bytes) =
        (Duration::ZERO, Duration::ZERO, Duration::ZERO, 0);
    let before = winnow_stats();
    let mut parses = 0u64;
    let mut benched = 0;
    for file in files {
        let Ok(source) = std::fs::read(file) else {
            continue;
        };
        if clean && !parse_file(engine_state, file, &source, false).0.parse_errors.is_empty() {
            continue;
        }
        benched += 1;
        bytes += source.len();
        let syntax = std::str::from_utf8(&source).map_or(Duration::ZERO, |text| {
            syntax_time(engine_state, text, iters * 3)
        });
        syntax_total += syntax;
        let time = |winnow: bool, iters: u32| {
            let start = Instant::now();
            for _ in 0..iters {
                let (working_set, block) = parse_file(engine_state, file, &source, winnow);
                std::hint::black_box((working_set.parse_errors.len(), block));
            }
            start.elapsed() / iters.max(1)
        };
        time(false, iters.min(5));
        time(true, iters.min(5));
        let (mut classic_rounds, mut winnow_rounds) = (Vec::new(), Vec::new());
        for _ in 0..ROUNDS {
            classic_rounds.push(time(false, iters));
            winnow_rounds.push(time(true, iters));
        }
        parses += u64::from(iters) * ROUNDS as u64 + u64::from(iters.min(5));
        classic_rounds.sort();
        winnow_rounds.sort();
        let (classic, winnow) = (classic_rounds[ROUNDS / 2], winnow_rounds[ROUNDS / 2]);
        classic_total += classic;
        winnow_total += winnow;
        println!(
            "{:<60} classic {:>9.1?}  winnow {:>9.1?} (syntax {:>9.1?})  {:>6.2}x",
            file.display(),
            classic,
            winnow,
            syntax,
            classic.as_secs_f64() / winnow.as_secs_f64().max(1e-12)
        );
    }
    let after = winnow_stats();
    println!(
        "TOTAL {benched} files, {bytes} bytes: classic {classic_total:.2?}, winnow {winnow_total:.2?} (of which winnow syntax {syntax_total:.2?}), {:.3}x (medians of {ROUNDS} rounds)",
        classic_total.as_secs_f64() / winnow_total.as_secs_f64().max(1e-12)
    );
    let per = |a: u64, b: u64| (a - b) / parses.max(1) * benched as u64;
    println!(
        "winnow front end per parse of the set: {} statements lowered, {} parsed by the classic parser ({} bytes), {} blocks by winnow, {} blocks classic",
        per(after.lowered_statements, before.lowered_statements),
        per(after.classic_statements, before.classic_statements),
        per(after.classic_statement_bytes, before.classic_statement_bytes),
        per(after.winnow_blocks, before.winnow_blocks),
        per(after.classic_blocks, before.classic_blocks),
    );
}

/// The parse as text, every id resolved: the block, the declarations the parse added, the
/// errors, and the highlighting.
fn render(working_set: &StateWorkingSet, block: &Block) -> String {
    let mut renderer = Renderer {
        working_set,
        out: String::new(),
        depth: 0,
        rendering: HashSet::new(),
    };
    renderer.line("BLOCK");
    renderer.block(block);
    let permanent = working_set.permanent_state.num_decls();
    let mut decls: Vec<(String, nu_protocol::DeclId)> = (permanent..working_set.num_decls())
        .map(nu_protocol::DeclId::new)
        .map(|id| (working_set.get_decl(id).name().to_string(), id))
        .collect();
    decls.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, id) in decls {
        let decl = working_set.get_decl(id);
        renderer.line(&format!("DECL {name} {:?}", decl.command_type()));
        renderer.signature(&decl.signature());
        if let Some(block_id) = decl.block_id() {
            renderer.block_id(block_id);
        }
    }
    for error in &working_set.parse_errors {
        renderer.line(&format!("PARSE_ERROR {error:?}"));
    }
    for error in &working_set.compile_errors {
        renderer.line(&format!("COMPILE_ERROR {error:?}"));
    }
    for (span, shape) in flatten_block(working_set, block) {
        renderer.line(&format!("FLAT {}..{} {shape}", span.start, span.end));
    }
    renderer.out
}

struct Renderer<'a, 'e> {
    working_set: &'a StateWorkingSet<'e>,
    out: String,
    depth: usize,
    /// Blocks being rendered, so a block reachable from itself is rendered once.
    rendering: HashSet<BlockId>,
}

impl Renderer<'_, '_> {
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "{:indent$}{text}", "", indent = self.depth * 2);
    }

    fn nested(&mut self, f: impl FnOnce(&mut Self)) {
        self.depth += 1;
        f(self);
        self.depth -= 1;
    }

    fn var(&self, var_id: VarId) -> String {
        match var_id {
            NU_VARIABLE_ID => "$nu".into(),
            IN_VARIABLE_ID => "$in".into(),
            ENV_VARIABLE_ID => "$env".into(),
            LAST_VARIABLE_ID => "$ans".into(),
            _ => {
                let var = self.working_set.get_variable(var_id);
                let name = var
                    .name
                    .as_deref()
                    .map(String::from_utf8_lossy)
                    .unwrap_or_default();
                format!(
                    "var({name}@{}..{}:{} mut={})",
                    var.declaration_span.start, var.declaration_span.end, var.ty, var.mutable
                )
            }
        }
    }

    fn signature(&mut self, signature: &Signature) {
        let text = format!(
            "SIG name={:?} desc={:?} extra={:?} io={:?} unknown_args={} req={:?} opt={:?} rest={:?} named={:?}",
            signature.name,
            signature.description,
            signature.extra_description,
            signature.input_output_types,
            signature.allows_unknown_args,
            signature
                .required_positional
                .iter()
                .map(|p| (p.name.clone(), p.shape.to_string(), p.var_id.map(|v| self.var(v)), p.desc.clone(), p.default_value.clone()))
                .collect::<Vec<_>>(),
            signature
                .optional_positional
                .iter()
                .map(|p| (p.name.clone(), p.shape.to_string(), p.var_id.map(|v| self.var(v)), p.desc.clone(), p.default_value.clone()))
                .collect::<Vec<_>>(),
            signature
                .rest_positional
                .as_ref()
                .map(|p| (p.name.clone(), p.shape.to_string(), p.var_id.map(|v| self.var(v)), p.desc.clone())),
            signature
                .named
                .iter()
                .map(|f| (f.long.clone(), f.short, f.arg.as_ref().map(|a| a.to_string()), f.var_id.map(|v| self.var(v)), f.desc.clone(), f.default_value.clone()))
                .collect::<Vec<_>>(),
        );
        self.line(&text);
    }

    fn block_id(&mut self, block_id: BlockId) {
        if !self.rendering.insert(block_id) {
            self.line("(block being rendered)");
            return;
        }
        let block = self.working_set.get_block(block_id).clone();
        self.block(&block);
        self.rendering.remove(&block_id);
    }

    fn block(&mut self, block: &Block) {
        let mut captures: Vec<String> = block.captures.iter().map(|(v, s)| format!("{}@{}..{}", self.var(*v), s.start, s.end)).collect();
        captures.sort();
        self.line(&format!(
            "span={:?} scoped={} redirect_env={} compiled={} captures={captures:?}",
            block.span,
            block.parsed_scoped,
            block.redirect_env,
            block.ir_block.is_some(),
        ));
        self.signature(&block.signature.clone());
        for pipeline in &block.pipelines {
            self.line("PIPELINE");
            self.nested(|this| {
                for element in &pipeline.elements {
                    this.line(&format!("ELEMENT pipe={:?}", element.pipe));
                    this.nested(|this| {
                        this.expression(&element.expr);
                        if let Some(redirection) = &element.redirection {
                            this.redirection(redirection);
                        }
                    });
                }
            });
        }
    }

    fn redirection(&mut self, redirection: &PipelineRedirection) {
        let target = |this: &mut Self, label: &str, target: &RedirectionTarget| match target {
            RedirectionTarget::File { expr, append, span } => {
                this.line(&format!("REDIRECT {label} file append={append} {span:?}"));
                this.nested(|this| this.expression(expr));
            }
            RedirectionTarget::Pipe { span } => this.line(&format!("REDIRECT {label} pipe {span:?}")),
        };
        match redirection {
            PipelineRedirection::Single { source, target: t } => target(self, &format!("{source:?}"), t),
            PipelineRedirection::Separate { out, err } => {
                target(self, "out", out);
                target(self, "err", err);
            }
        }
    }

    fn expression(&mut self, expression: &Expression) {
        let head = format!("{}..{} :{}", expression.span.start, expression.span.end, expression.ty);
        match &expression.expr {
            Expr::Var(var_id) => self.line(&format!("Var {} {head}", self.var(*var_id))),
            Expr::VarDecl(var_id) => self.line(&format!("VarDecl {} {head}", self.var(*var_id))),
            Expr::Call(call) => {
                let mut info: Vec<String> = call
                    .parser_info
                    .iter()
                    .map(|(k, v)| match &v.expr {
                        // The constants are variable ids; show what they name.
                        Expr::ImportPattern(pattern) => format!(
                            "{k}={:?} {:?} {:?}",
                            pattern.head,
                            pattern.members,
                            pattern.constants.iter().map(|c| self.var(*c)).collect::<Vec<_>>()
                        ),
                        other => format!("{k}={other:?}"),
                    })
                    .collect();
                info.sort();
                self.line(&format!(
                    "Call {} head={}..{} {head} info={info:?}",
                    self.working_set.get_decl(call.decl_id).name(),
                    call.head.start,
                    call.head.end
                ));
                self.nested(|this| {
                    for argument in &call.arguments {
                        match argument {
                            Argument::Positional(e) => {
                                this.line("Positional");
                                this.nested(|this| this.expression(e));
                            }
                            Argument::Unknown(e) => {
                                this.line("Unknown");
                                this.nested(|this| this.expression(e));
                            }
                            Argument::Spread(e) => {
                                this.line("Spread");
                                this.nested(|this| this.expression(e));
                            }
                            Argument::Named((long, short, value)) => {
                                this.line(&format!("Named {long:?} {short:?}"));
                                if let Some(value) = value {
                                    this.nested(|this| this.expression(value));
                                }
                            }
                        }
                    }
                });
            }
            Expr::ExternalCall(call_head, arguments) => {
                self.line(&format!("ExternalCall {head}"));
                self.nested(|this| {
                    this.expression(call_head);
                    for argument in arguments.iter() {
                        match argument {
                            ExternalArgument::Regular(e) => this.expression(e),
                            ExternalArgument::Spread(e) => {
                                this.line("Spread");
                                this.nested(|this| this.expression(e));
                            }
                        }
                    }
                });
            }
            Expr::RowCondition(id) | Expr::Subexpression(id) | Expr::Block(id) | Expr::Closure(id) => {
                let kind = match &expression.expr {
                    Expr::RowCondition(_) => "RowCondition",
                    Expr::Subexpression(_) => "Subexpression",
                    Expr::Block(_) => "Block",
                    _ => "Closure",
                };
                self.line(&format!("{kind} {head}"));
                let id = *id;
                self.nested(|this| this.block_id(id));
            }
            Expr::UnaryNot(inner) => {
                self.line(&format!("UnaryNot {head}"));
                self.nested(|this| this.expression(inner));
            }
            Expr::BinaryOp(lhs, op, rhs) => {
                self.line(&format!("BinaryOp {head}"));
                self.nested(|this| {
                    this.expression(lhs);
                    this.expression(op);
                    this.expression(rhs);
                });
            }
            Expr::Collect(var_id, inner) => {
                self.line(&format!("Collect {} {head}", self.var(*var_id)));
                self.nested(|this| this.expression(inner));
            }
            Expr::MatchBlock(arms) => {
                self.line(&format!("MatchBlock {head}"));
                self.nested(|this| {
                    for (pattern, result) in arms {
                        this.pattern(pattern);
                        this.nested(|this| this.expression(result));
                    }
                });
            }
            Expr::List(items) => {
                self.line(&format!("List {head}"));
                self.nested(|this| {
                    for item in items {
                        match item {
                            ListItem::Item(e) => this.expression(e),
                            ListItem::Spread(span, e) => {
                                this.line(&format!("Spread {span:?}"));
                                this.nested(|this| this.expression(e));
                            }
                        }
                    }
                });
            }
            Expr::Table(table) => {
                self.line(&format!("Table {head}"));
                self.nested(|this| {
                    for column in table.columns.iter() {
                        this.expression(column);
                    }
                    for row in table.rows.iter() {
                        this.line("Row");
                        this.nested(|this| row.iter().for_each(|e| this.expression(e)));
                    }
                });
            }
            Expr::Record(items) => {
                self.line(&format!("Record {head}"));
                self.nested(|this| {
                    for item in items {
                        match item {
                            RecordItem::Pair(key, value) => {
                                this.expression(key);
                                this.nested(|this| this.expression(value));
                            }
                            RecordItem::Spread(span, e) => {
                                this.line(&format!("Spread {span:?}"));
                                this.nested(|this| this.expression(e));
                            }
                        }
                    }
                });
            }
            Expr::Keyword(keyword) => {
                self.line(&format!("Keyword {} {:?} {head}", String::from_utf8_lossy(&keyword.keyword), keyword.span));
                self.nested(|this| this.expression(&keyword.expr));
            }
            Expr::ValueWithUnit(value) => {
                self.line(&format!("ValueWithUnit {:?} {head}", value.unit));
                self.nested(|this| this.expression(&value.expr));
            }
            Expr::FullCellPath(path) => {
                self.line(&format!("FullCellPath {:?} {head}", path.tail));
                self.nested(|this| this.expression(&path.head));
            }
            Expr::StringInterpolation(parts) | Expr::GlobInterpolation(parts, _) => {
                let kind = match &expression.expr {
                    Expr::GlobInterpolation(_, quoted) => format!("GlobInterpolation quoted={quoted}"),
                    _ => "StringInterpolation".to_string(),
                };
                self.line(&format!("{kind} {head}"));
                self.nested(|this| parts.iter().for_each(|e| this.expression(e)));
            }
            Expr::Signature(signature) => {
                self.line(&format!("Signature {head}"));
                self.nested(|this| this.signature(signature));
            }
            Expr::AttributeBlock(block) => {
                self.line(&format!("AttributeBlock {head}"));
                self.nested(|this| {
                    for attribute in &block.attributes {
                        this.expression(&attribute.expr);
                    }
                    this.expression(&block.item);
                });
            }
            Expr::Range(range) => {
                self.line(&format!("Range {:?} {head}", range.operator));
                self.nested(|this| {
                    for part in [&range.from, &range.next, &range.to] {
                        match part {
                            Some(e) => this.expression(e),
                            None => this.line("None"),
                        }
                    }
                });
            }
            other => self.line(&format!("{other:?} {head}")),
        }
    }

    fn pattern(&mut self, pattern: &MatchPattern) {
        let head = format!("{}..{}", pattern.span.start, pattern.span.end);
        match &pattern.pattern {
            Pattern::Variable(var_id) => self.line(&format!("PatVariable {} {head}", self.var(*var_id))),
            Pattern::Rest(var_id) => self.line(&format!("PatRest {} {head}", self.var(*var_id))),
            Pattern::List(items) | Pattern::Or(items) => {
                self.line(&format!("PatList/Or {head}"));
                self.nested(|this| items.iter().for_each(|p| this.pattern(p)));
            }
            Pattern::Record(fields) => {
                self.line(&format!("PatRecord {head}"));
                self.nested(|this| {
                    for (name, p) in fields {
                        this.line(name);
                        this.pattern(p);
                    }
                });
            }
            Pattern::Expression(e) => {
                self.line(&format!("PatExpression {head}"));
                self.nested(|this| this.expression(e));
            }
            other => self.line(&format!("{other:?} {head}")),
        }
        if let Some(guard) = &pattern.guard {
            self.nested(|this| {
                this.line("Guard");
                this.expression(guard);
            });
        }
    }
}
