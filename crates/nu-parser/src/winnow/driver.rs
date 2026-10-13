//! Parsing a block statement by statement with the winnow parser.
//!
//! [`parse_block`] takes the place of the classic parser's `parse_block` for a whole block: it
//! hands the block's definitions, then each of its statements, to a [`Driver`], which lowers
//! them into the working set or has the classic parser parse them.
//!
//! The statements of a long block are parsed on a second thread, ahead of the lowering on this
//! one ([`parse_ahead`]): the winnow parser needs only the source and the command names, and
//! the lowering, which changes the working set, takes each statement as it comes. The thread
//! resolves names with a copy of the working set's ([`NamesSnapshot`]), made when it starts and
//! again after each statement that changes which commands exist (`use`, `overlay`, `hide`,
//! `source`, `run`), up to which it parses. Every answer it got is checked against the live
//! working set before its statement is lowered, so the tree is the one parsing on this thread
//! would have built; when an answer differs, the classic parser takes the rest of the block.
//!
//! ```text
//! this thread                                   second thread
//! -----------                                   -------------
//! BlockStatements::new: lex, scan definitions
//! parse_ahead:
//!   NamesSnapshot::new
//!   spawn -------------------------------------> statements.parse with an AskedLookup:
//!   declare the block's definitions               parse a statement, its answers recorded
//!   receive a statement <----------------------- send it with its answers
//!     an answer differs now: set `stop`, the      go on unless the statement changes names
//!       classic parser takes the rest of the      or has a syntax error, the block ends, or
//!       block                                     `stop` is set
//!     a syntax error: the classic parser takes    drop the sender
//!       the rest of the block
//!     else lower it
//!   ...until the sender is dropped
//! the next statement changes names:
//!   parse and lower it here
//! parse_ahead again, with a new copy of the names
//! ...
//! ```
//!
//! The statements of a block once less than [`AHEAD_MIN_BYTES`] of it is left to parse, and
//! every block where no second thread can start, are parsed and lowered on this thread alone.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
        mpsc,
    },
};

use nu_protocol::{
    Module, Span, Type,
    ast::{Block, Pipeline},
    engine::StateWorkingSet,
};
use nu_winnow_parser::{
    BlockSink, BlockStatements, Definitions, Diagnostic, PredeclaredDef, ast as w,
};

use super::{
    lookup::{Asked, AskedLookup, EngineLookup, NamesSnapshot, changed_answer},
    lower::{Lower, Predecl, StatementTarget, parse_classic},
    stats,
};
use crate::{
    lex::lex, lite_parser::lite_parse, parse_def::parse_def_predecl, parse_pipelines::finish_block,
};

/// What kind of block is parsed, with what the kind needs.
pub(super) enum BlockKind {
    /// An ordinary block (a file, or the body of a closure or command), as the classic
    /// parser's `parse_block` takes it. The statements of a subexpression (`is_subexpression`)
    /// are lexed as `parse_full_cell_path` lexes a parenthesized one: newlines as whitespace.
    Block {
        scoped: bool,
        is_subexpression: bool,
        input_type: Option<Type>,
    },
    /// A module's body: its statements define the module's exports.
    Module { name: Vec<u8> },
}

/// Parse the block covering `span` with the winnow parser, its statements in `inner` (the whole
/// span, or what is inside a block's braces). `source` is the text around it, its first byte at
/// `offset` in the working set's span space. Returns the block and, for a module body, the
/// module; `None` when the winnow parser cannot take the block, with the working set left as it
/// was.
pub(super) fn parse_block(
    working_set: &mut StateWorkingSet,
    source: &str,
    offset: usize,
    span: Span,
    inner: Span,
    kind: BlockKind,
) -> Option<(Block, Option<Module>)> {
    let local_span = nu_winnow_parser::Span::new(inner.start - offset, inner.end - offset);
    let target = match kind {
        BlockKind::Block {
            scoped,
            is_subexpression,
            input_type,
        } => Target::Block {
            scoped,
            is_subexpression,
            input_type,
        },
        BlockKind::Module { name } => Target::Module {
            module: Box::new(Module::from_span(name.clone(), span)),
            name,
        },
    };
    let enters_scope = match &target {
        Target::Block { scoped, .. } => *scoped,
        Target::Module { .. } => true,
    };
    if enters_scope {
        working_set.enter_scope();
    }

    let block_errors = working_set.parse_errors.len();
    let cell = RefCell::new(working_set);
    let lookup = EngineLookup::new(&cell);
    let mut driver = Driver {
        working_set: &cell,
        source,
        offset,
        span,
        inner,
        target,
        pipelines: Vec::new(),
        block_errors,
    };
    let result = parse_statements(source, local_span, &lookup, &mut driver);
    let Driver {
        pipelines, target, ..
    } = driver;
    drop(lookup);
    let working_set = cell.into_inner();

    if result.is_err() {
        if enters_scope {
            working_set.exit_scope();
        }
        stats::record_classic_block();
        return None;
    }
    stats::record_winnow_block();

    let mut block = Block::new_with_capacity(pipelines.len());
    block.pipelines = pipelines;
    match target {
        Target::Block {
            scoped,
            is_subexpression,
            input_type,
        } => {
            let block = finish_block(
                working_set,
                block,
                span,
                scoped,
                is_subexpression,
                input_type.as_ref(),
            );
            Some((block, None))
        }
        Target::Module { module, .. } => {
            block.span = Some(span);
            working_set.exit_scope();
            Some((block, Some(*module)))
        }
    }
}

/// The least text the statements still to parse must have for a second thread to parse them
/// ahead of the lowering: starting the thread costs about what parsing 1 kB does.
const AHEAD_MIN_BYTES: usize = 4096;

/// Parse the statements of the block covering `span` of `source` (the winnow parser's span
/// space) and hand them to `driver`: first the block's definitions, then each statement, on
/// this thread or, a run at a time, ahead of the lowering on a second one ([`parse_ahead`]).
/// Fails only when [`BlockStatements::new`] rejects the block, before anything was handed to
/// `driver`.
fn parse_statements(
    source: &str,
    span: nu_winnow_parser::Span,
    lookup: &EngineLookup,
    driver: &mut Driver,
) -> Result<(), Vec<Diagnostic>> {
    // A thread that has slept takes tens of microseconds to wake up: wake one now, so that it
    // is still awake (looking for work) when `parse_ahead` hands it the statements, once the
    // block is lexed. The whole block's length stands in for `remaining()`, which the loop
    // below tests and which is known only after lexing.
    let threads = ahead_threads();
    if let Some(threads) = threads
        && span.len() >= AHEAD_MIN_BYTES
        && AheadThread::any_free()
    {
        threads.spawn(|| {});
    }
    // A subexpression's statements are lexed as the classic `parse_full_cell_path` lexes a
    // parenthesized one: newlines as whitespace.
    let subexpression = matches!(
        driver.target,
        Target::Block {
            is_subexpression: true,
            ..
        }
    );
    let (mut statements, definitions) = BlockStatements::new(source, span, lookup, subexpression)?;
    // Declared before the first statement is lowered: by `parse_ahead` once its thread has
    // started, so that the thread parses the first statements meanwhile, or else before the
    // first statement parsed here. `None` once declared.
    let mut definitions = Some(definitions);
    // Each turn hands `driver` a run of statements, parsed ahead or here. `go_on` turns false
    // when the driver has handed the rest of the block to the classic parser.
    let mut go_on = true;
    while go_on && !statements.is_done() {
        let short = statements.remaining().len() < AHEAD_MIN_BYTES;
        go_on = if let Some(threads) = threads
            && !short
            && !changes_names(statements.next_words(source))
            && let Some(_thread) = AheadThread::take()
        {
            let definitions = definitions.take().unwrap_or_default();
            parse_ahead(
                threads,
                source,
                span,
                &mut statements,
                definitions,
                lookup,
                driver,
            )
        } else {
            if let Some(definitions) = definitions.take() {
                declare(definitions, source, span, lookup, driver);
            }
            // On this thread: all that is left when it is short or there is no second thread
            // (the next turn would decide the same), else only the next statement: one that
            // changes the command names, so a run parsed ahead would end right after it, or one
            // parsed while both threads run other parses' runs, after which a thread may be
            // free. The sink then stops after that statement, and this `go_on` carries the
            // driver's answer to the loop, which goes on with a run that sees the new names.
            let all = short || threads.is_none();
            let mut go_on = true;
            statements.parse(source, lookup, &mut |pipeline, diagnostics, _| {
                go_on = driver.statement(pipeline, diagnostics);
                go_on && all
            });
            go_on
        };
    }
    // The loop takes them on its first turn: still here only for an empty block.
    if let Some(definitions) = definitions.take() {
        declare(definitions, source, span, lookup, driver);
    }
    Ok(())
}

/// Declare the block's `definitions` in the working set (what the classic parser's
/// `parse_block` does with `parse_def_predecl`), their signatures parsed in `source`.
fn declare(
    definitions: Definitions,
    source: &str,
    span: nu_winnow_parser::Span,
    lookup: &EngineLookup,
    driver: &mut Driver,
) {
    for def in definitions.parse(source, span, lookup) {
        driver.predecl(def);
    }
}

/// A statement the second thread parsed, with what it reported, the questions about command
/// names it asked and the bound on name lengths it looked up names within.
struct Ahead<'a> {
    pipeline: w::Pipeline<'a>,
    diagnostics: Vec<Diagnostic>,
    asked: Vec<Asked>,
    longest_name: usize,
}

/// Parse statements on a second thread while this one declares the block's `definitions` and
/// lowers the statements as they arrive, until one that changes which command names exist (the
/// thread's copy of them is then out of date), one with a syntax error (the classic parser
/// takes the rest of the block from it) or the end of the block (`span`). Returns whether to go
/// on with the block: `false` once the classic parser has taken the rest of it.
///
/// The thread's names are copied before the definitions are declared, so that it starts at
/// once: it never asks about the block's own definitions, which the winnow parser declared in
/// its own scopes ([`BlockStatements::new`]) and finds there first.
///
/// Before a statement is lowered, every question the thread asked about command names is asked
/// again of the live working set ([`changed_answer`]), at the point where parsing on this thread
/// would have asked it. When an answer differs, the classic parser takes the rest of the block
/// (the thread has parsed on past the statement with the old names, so the winnow parser cannot
/// go on from it); `stop` tells the thread to stop first, and what it sent meanwhile is
/// dropped.
fn parse_ahead(
    threads: &rayon::ThreadPool,
    source: &str,
    span: nu_winnow_parser::Span,
    statements: &mut BlockStatements,
    definitions: Definitions,
    lookup: &EngineLookup,
    driver: &mut Driver,
) -> bool {
    let names = NamesSnapshot::new(lookup);
    // Set once this thread takes no more statements. It only saves the thread work (the
    // statements themselves go through the channel), so `Relaxed` is enough.
    let stop = &AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel::<Ahead>();
    // `in_place_scope` runs the closure on this thread and returns once the spawned task is
    // done, so the task can borrow `statements`, `source` and `stop`.
    threads.in_place_scope(|scope| {
        // The thread owns `sender`: dropping it when done ends the loop below. The loop holds
        // the receiver until then, so a `send` fails only while this thread unwinds from a
        // panic, and the thread then stops too.
        scope.spawn(move |_| {
            let asked = Rc::new(RefCell::new(Vec::new()));
            let lookup = AskedLookup {
                names,
                asked: Rc::clone(&asked),
            };
            statements.parse(
                source,
                lookup,
                &mut |pipeline, diagnostics, longest_name| {
                    let changes = statement_changes_names(&pipeline);
                    // The classic parser takes the rest of the block from a statement with a
                    // syntax error (`Driver::take_statement`): what would be parsed after it is
                    // dropped.
                    let failed = !diagnostics.is_empty();
                    // The sink is called between statements: what was asked since the last call
                    // was asked while parsing this one.
                    let asked = asked.take();
                    let sent = sender
                        .send(Ahead {
                            pipeline,
                            diagnostics,
                            asked,
                            longest_name,
                        })
                        .is_ok();
                    sent && !changes && !failed && !stop.load(Relaxed)
                },
            );
        });
        stats::record_ahead_run();
        declare(definitions, source, span, lookup, driver);
        let mut go_on = true;
        // Until the thread is done and drops its sender; after a stop, what it parsed
        // meanwhile is dropped (the classic parser has parsed it).
        for ahead in receiver {
            if !go_on {
                continue;
            }
            if let Some(changed) = changed_answer(lookup, &ahead.asked, ahead.longest_name) {
                // Stopped first, so that the thread does not go on parsing what the classic
                // parser parses now.
                stop.store(true, Relaxed);
                stats::record_ahead_fallback(&changed);
                driver.rest_to_classic(&ahead.pipeline, None);
                go_on = false;
            } else {
                // `false` after a statement with a syntax error, after which the thread
                // stopped by itself.
                go_on = driver.take_statement(&ahead.pipeline, &ahead.diagnostics);
            }
        }
        go_on
    })
}

/// Whether a statement whose first words are `words` changes which command names exist:
/// `use`, `export use`, `overlay ...`, `hide`, `source`, `source-env`, `run`, `plugin use`, the
/// statements the classic `parse_builtin_commands` tells by their first word. A `def` or an
/// `alias` does not count: the winnow parser declares those names itself.
///
/// A wrong answer costs only time: here a run ends early, or goes on past a statement this
/// misses (an alias of `overlay use`) and [`changed_answer`] catches any later answer it
/// changed; in a nested block (`changes_commands` in `lower`), the block is parsed again for
/// nothing, or the lowering catches the alias once the classic parser has parsed it.
fn changes_names(words: (Option<&str>, Option<&str>)) -> bool {
    matches!(
        words,
        (
            Some("use" | "overlay" | "hide" | "source" | "source-env" | "run"),
            _
        ) | (Some("export" | "plugin"), Some("use"))
    )
}

/// [`changes_names`] for a parsed statement, from the first words of its commands' names: the
/// classic parser takes `source me` for a `source` even where a command `source me` exists.
/// The lowering of a nested block asks it too (`changes_commands` in `lower`).
pub(super) fn statement_changes_names(pipeline: &w::Pipeline<'_>) -> bool {
    pipeline.elements.iter().any(|element| {
        changes_names(match &element.expr.expr {
            w::Expr::Use(_) => (Some("use"), None),
            w::Expr::Export(export) if matches!(export.item.expr, w::Expr::Use(_)) => {
                (Some("export"), Some("use"))
            }
            w::Expr::Call(call) => {
                let mut words = call.head.name.split(' ');
                (words.next(), words.next())
            }
            _ => (None, None),
        })
    })
}

/// How many threads [`ahead_threads`] has.
const AHEAD_THREADS: usize = 2;

/// The stack of each of [`ahead_threads`]: the 8 MiB the main thread gets on Linux and macOS,
/// not the 2 MiB of a spawned thread. The syntax of a deeply nested block is parsed there,
/// and a stack overflow aborts the process. Per level of nesting the winnow parser takes less
/// stack than the classic parser, so the main thread runs out first, also on Windows, whose
/// main thread nushell gives 10 MB.
const AHEAD_STACK_SIZE: usize = 8 * 1024 * 1024;

/// The threads that parse statements ahead of the lowering, started on first use and kept, so
/// that a run starts in microseconds, on a thread whose caches hold the parser. They are not
/// rayon's global pool: a parse inside a `par-each` closure waits for its run, which must not
/// queue behind the closures. A run starts only on a free thread ([`AheadThread`]), so it never
/// waits for one: while both run other parses' runs (concurrent parses, or a module's body
/// loaded by the lowering of a statement of a run), a block is parsed on its own thread.
/// `None` on a machine that runs one thread at a time, or where threads cannot be started.
fn ahead_threads() -> Option<&'static rayon::ThreadPool> {
    static THREADS: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    THREADS
        .get_or_init(|| {
            let parallel = std::thread::available_parallelism().is_ok_and(|cpus| cpus.get() > 1);
            parallel
                .then(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(AHEAD_THREADS)
                        .stack_size(AHEAD_STACK_SIZE)
                        .thread_name(|_| "nu-winnow-ahead".into())
                        .build()
                        .ok()
                })
                .flatten()
        })
        .as_ref()
}

/// One of [`ahead_threads`] taken for a run, given back when dropped.
struct AheadThread;

/// How many of [`ahead_threads`] are taken.
static AHEAD_THREADS_TAKEN: AtomicUsize = AtomicUsize::new(0);

impl AheadThread {
    /// Take a free thread, if there is one.
    fn take() -> Option<Self> {
        AHEAD_THREADS_TAKEN
            .fetch_update(Relaxed, Relaxed, |taken| {
                (taken < AHEAD_THREADS).then_some(taken + 1)
            })
            .ok()
            .map(|_| AheadThread)
    }

    /// Whether a thread is free now.
    fn any_free() -> bool {
        AHEAD_THREADS_TAKEN.load(Relaxed) < AHEAD_THREADS
    }
}

impl Drop for AheadThread {
    fn drop(&mut self) {
        AHEAD_THREADS_TAKEN.fetch_sub(1, Relaxed);
    }
}

/// Where the statements of a block go; [`statement_target`] lends it to each statement as a
/// [`StatementTarget`].
enum Target {
    /// An ordinary block, as [`BlockKind::Block`] describes it; the first statement receives
    /// the block's input.
    Block {
        scoped: bool,
        is_subexpression: bool,
        input_type: Option<Type>,
    },
    /// A module body; the statements add to `module`.
    Module { module: Box<Module>, name: Vec<u8> },
}

/// Receives the winnow parser's statements and turns them into pipelines.
struct Driver<'c, 'w, 'e, 's> {
    /// The working set, in the cell the block's [`EngineLookup`] reads it through while a
    /// statement is parsed on this thread; borrowed mutably only while a definition or a
    /// statement is handed over.
    working_set: &'c RefCell<&'w mut StateWorkingSet<'e>>,
    /// The text the winnow parser parses, which starts at `offset` in the working set's span
    /// space.
    source: &'s str,
    offset: usize,
    /// The block's span, which errors about the whole block point at; for a block in braces
    /// (a closure's body parsed again by `Lower::block`), the braces included.
    span: Span,
    /// The span of the block's statements: `span` without the braces of a block in braces.
    inner: Span,
    target: Target,
    /// The block's pipelines so far.
    pipelines: Vec<Pipeline>,
    /// Where the block's errors start, past the lex and lite-parse errors the classic parser
    /// put there ([`parse_classic`]).
    block_errors: usize,
}

impl Driver<'_, '_, '_, '_> {
    /// A span of the winnow parser's source, in the working set's span space.
    fn global(&self, span: nu_winnow_parser::Span) -> Span {
        Span::new(self.offset + span.start, self.offset + span.end)
    }

    /// Lower the next statement, `pipeline`, which parsing reported `diagnostics` in (see
    /// [`BlockSink::statement`]). Returns whether to go on with the next statement: `false`
    /// once the classic parser has taken the rest of the block.
    fn take_statement(&mut self, pipeline: &w::Pipeline<'_>, diagnostics: &[Diagnostic]) -> bool {
        if let Some(error) = diagnostics.first() {
            // A syntax error: the classic parser takes the rest of the block, reporting the
            // error as it always has.
            self.rest_to_classic(pipeline, Some(error));
            return false;
        }
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let mut lower = Lower::new(working_set, self.source, self.offset);
        let span = lower.statement_span(pipeline);
        let mut target = statement_target(&mut self.target, &self.pipelines, self.span);
        lower.statement_or_classic(
            pipeline,
            span,
            &mut target,
            &mut self.pipelines,
            &mut self.block_errors,
        );
        true
    }

    /// Hand the rest of the block, from `pipeline` on, to the classic parser. `error`: the
    /// first error the winnow parser reported in `pipeline`, if it reported one.
    fn rest_to_classic(&mut self, pipeline: &w::Pipeline<'_>, error: Option<&Diagnostic>) {
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let lower = Lower::new(working_set, self.source, self.offset);
        let start = lower.statement_span(pipeline).start;
        let mut target = statement_target(&mut self.target, &self.pipelines, self.span);
        // Where the statement ends is the classic parser's to decide, so it parses the rest
        // of the block's statements (up to a closing brace, not through it).
        let span = Span::new(start, self.inner.end);
        match error {
            Some(error) => stats::record_error_statement(span, error, lower.working_set),
            None => stats::record_classic_statement(span.len(), false),
        }
        parse_classic(
            lower.working_set,
            span,
            &mut target,
            &mut self.pipelines,
            &mut self.block_errors,
        );
    }
}

/// The statement target for the next statement of a block that has `pipelines` so far.
fn statement_target<'t>(
    target: &'t mut Target,
    pipelines: &[Pipeline],
    span: Span,
) -> StatementTarget<'t> {
    match target {
        Target::Block {
            input_type,
            is_subexpression,
            ..
        } => StatementTarget::Block {
            input_type: if pipelines.is_empty() {
                input_type.as_ref()
            } else {
                None
            },
            is_subexpression: *is_subexpression,
        },
        Target::Module { module, name } => StatementTarget::Module { name, module, span },
    }
}

impl<'a> BlockSink<'a> for Driver<'_, '_, '_, '_> {
    fn predecl(&mut self, def: PredeclaredDef<'a>) {
        let span = self.global(def.span);
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let mut lower = Lower::new(working_set, self.source, self.offset);
        let lowered = lower.predecl(Predecl {
            name: &def.name.item,
            name_span: def.name.span,
            signature: def.signature.as_ref(),
            wrapped: def.wrapped,
            external: def.external,
            statement_end: def.span.end,
        });
        if lowered {
            return;
        }
        // The classic predeclaration, from the statement's text: of a pipeline of one command,
        // as the classic `parse_block` does it.
        let (tokens, _) = lex(
            working_set.get_span_contents(span),
            span.start,
            &[],
            &[],
            false,
        );
        let (lite_block, _) = lite_parse(&tokens, working_set);
        for pipeline in &lite_block.block {
            if let [lite_command] = pipeline.commands.as_slice() {
                parse_def_predecl(working_set, lite_command.command_parts());
            }
        }
    }

    fn statement(&mut self, pipeline: w::Pipeline<'a>, diagnostics: Vec<Diagnostic>) -> bool {
        self.take_statement(&pipeline, &diagnostics)
    }
}
