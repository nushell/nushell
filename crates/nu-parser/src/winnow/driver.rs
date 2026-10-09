//! Parsing a block statement by statement with the winnow parser.
//!
//! The statements of a long block are parsed on a second thread, ahead of the lowering on this
//! one ([`parse_ahead`]): the winnow parser needs only the source and the command names, and
//! the lowering, which changes the working set, takes each statement as it comes. The thread
//! resolves names with a copy of the working set's ([`NamesSnapshot`]), made when it starts and
//! again after each statement that changes which commands exist (`use`, `overlay`, `hide`,
//! `source`), up to which it parses. Every answer it got is checked against the live working
//! set before its statement is lowered, so the tree is the one parsing on this thread would
//! have built; when an answer differs, the classic parser takes the rest of the block.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering::Relaxed},
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
    /// parser's `parse_block` takes it.
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
    let enters_scope = match &kind {
        BlockKind::Block { scoped, .. } => *scoped,
        BlockKind::Module { .. } => true,
    };
    if enters_scope {
        working_set.enter_scope();
    }

    let cell = RefCell::new(working_set);
    let lookup = EngineLookup::new(&cell);
    let mut driver = Driver {
        working_set: &cell,
        source,
        offset,
        span,
        target: match &kind {
            BlockKind::Block { input_type, .. } => Target::Block {
                input_type: input_type.clone(),
            },
            BlockKind::Module { name } => Target::Module {
                module: Box::new(Module::from_span(name.clone(), span)),
                name: name.clone(),
            },
        },
        pipelines: Vec::new(),
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
    match (kind, target) {
        (
            BlockKind::Block {
                scoped,
                is_subexpression,
                input_type,
            },
            _,
        ) => {
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
        (BlockKind::Module { .. }, Target::Module { module, .. }) => {
            block.span = Some(span);
            working_set.exit_scope();
            Some((block, Some(*module)))
        }
        (BlockKind::Module { .. }, Target::Block { .. }) => {
            working_set.exit_scope();
            None
        }
    }
}

/// The least text the statements still to parse must have for a second thread to parse them
/// ahead of the lowering: starting the thread costs about what parsing 1 kB does.
const AHEAD_MIN_BYTES: usize = 4096;

/// Parse the statements of the block covering `span` of `source` (the winnow parser's span
/// space) and hand them to `driver`: first the block's definitions, then each statement, on
/// this thread or, a run at a time, ahead of the lowering on a second one ([`parse_ahead`]).
fn parse_statements(
    source: &str,
    span: nu_winnow_parser::Span,
    lookup: &EngineLookup,
    driver: &mut Driver,
) -> Result<(), Vec<Diagnostic>> {
    // A thread that has slept takes tens of microseconds to wake up: wake one now, so that it
    // is still awake (looking for work) when `parse_ahead` hands it the statements, once the
    // block is lexed.
    let threads = ahead_threads();
    if let Some(threads) = threads
        && span.len() >= AHEAD_MIN_BYTES
    {
        threads.spawn(|| {});
    }
    let (mut statements, definitions) = BlockStatements::new(source, span, lookup)?;
    // Declared before the first statement is lowered: by `parse_ahead` once its thread has
    // started, so that the thread parses the first statements meanwhile.
    let mut definitions = Some(definitions);
    let mut go_on = true;
    while go_on && !statements.is_done() {
        let short = statements.remaining().len() < AHEAD_MIN_BYTES;
        go_on = if let Some(threads) = threads
            && !short
            && !changes_names(statements.next_words(source))
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
            // On this thread: all that is left when it is short or there is no second thread,
            // else the next statement, which changes the command names, so none after it can be
            // parsed ahead of it.
            let all = short || threads.is_none();
            let mut go_on = true;
            statements.parse(source, lookup, &mut |pipeline, diagnostics| {
                go_on = driver.statement(pipeline, diagnostics);
                go_on && all
            });
            go_on
        };
    }
    if let Some(definitions) = definitions.take() {
        declare(definitions, source, span, lookup, driver);
    }
    Ok(())
}

/// Declare the block's `definitions`, their signatures parsed in `source`.
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

/// A statement the second thread parsed, with what it reported and the questions about command
/// names it asked.
struct Ahead<'a> {
    pipeline: w::Pipeline<'a>,
    diagnostics: Vec<Diagnostic>,
    asked: Vec<Asked>,
}

/// Parse statements on a second thread while this one declares the block's `definitions` and
/// lowers the statements, until one that changes which command names exist (the thread's copy
/// of them is then out of date) or the end of the block (`span`). Returns whether to go on with
/// the block.
///
/// The thread's names are copied before the definitions are declared: it resolves the block's
/// own definitions in its own scopes, where the winnow parser declared them.
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
    let longest_name = names.longest_name;
    let stop = &AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel::<Ahead>();
    threads.in_place_scope(|scope| {
        // The thread owns `sender`: dropping it when done ends the loop below.
        scope.spawn(move |_| {
            let asked = Rc::new(RefCell::new(Vec::new()));
            let lookup = AskedLookup {
                names,
                asked: Rc::clone(&asked),
            };
            statements.parse(source, lookup, &mut |pipeline, diagnostics| {
                let changes = statement_changes_names(&pipeline);
                let asked = asked.take();
                let sent = sender
                    .send(Ahead {
                        pipeline,
                        diagnostics,
                        asked,
                    })
                    .is_ok();
                sent && !changes && !stop.load(Relaxed)
            });
        });
        stats::record_ahead_run();
        declare(definitions, source, span, lookup, driver);
        let mut go_on = true;
        // Until the thread is done and drops its sender; after a stop, what it parsed
        // meanwhile is dropped.
        for ahead in receiver {
            if !go_on {
                continue;
            }
            if let Some(changed) = changed_answer(lookup, &ahead.asked, longest_name) {
                stats::record_ahead_fallback(&changed);
                driver.rest_to_classic(&ahead.pipeline, false);
                go_on = false;
            } else {
                go_on = driver.take_statement(&ahead.pipeline, &ahead.diagnostics);
            }
            if !go_on {
                stop.store(true, Relaxed);
            }
        }
        go_on
    })
}

/// Whether a statement whose first words are `words` changes which command names exist:
/// `use`, `export use`, `overlay ...`, `hide`, `source`, `source-env`, `plugin use`. A `def`
/// or an `alias` does not count: the winnow parser declares those names itself.
fn changes_names(words: (Option<&str>, Option<&str>)) -> bool {
    matches!(
        words,
        (
            Some("use" | "overlay" | "hide" | "source" | "source-env"),
            _
        ) | (Some("export" | "plugin"), Some("use"))
    )
}

/// [`changes_names`] for a parsed statement.
fn statement_changes_names(pipeline: &w::Pipeline<'_>) -> bool {
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

/// The threads that parse statements ahead of the lowering, started on first use and kept, so
/// that a run starts in microseconds, on a thread whose caches hold the parser. They are not
/// rayon's global pool: a parse inside a `par-each` closure waits for its run, which must not
/// queue behind the closures. Two, so that a block parsed while another block's run goes on (a
/// module's body, loaded by the lowering of a statement of that run) does not wait for it.
/// `None` on a machine that runs one thread at a time, or where threads cannot be started.
fn ahead_threads() -> Option<&'static rayon::ThreadPool> {
    static THREADS: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    THREADS
        .get_or_init(|| {
            let parallel = std::thread::available_parallelism().is_ok_and(|cpus| cpus.get() > 1);
            parallel
                .then(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(2)
                        .thread_name(|_| "nu-winnow-ahead".into())
                        .build()
                        .ok()
                })
                .flatten()
        })
        .as_ref()
}

/// Where the statements of a block go.
enum Target {
    /// An ordinary block; the first statement receives the block's input.
    Block { input_type: Option<Type> },
    /// A module body; the statements add to `module`.
    Module { module: Box<Module>, name: Vec<u8> },
}

/// Receives the winnow parser's statements and turns them into pipelines.
struct Driver<'c, 'w, 'e, 's> {
    working_set: &'c RefCell<&'w mut StateWorkingSet<'e>>,
    source: &'s str,
    offset: usize,
    /// The block's span, which errors about the whole block point at.
    span: Span,
    target: Target,
    pipelines: Vec<Pipeline>,
}

impl Driver<'_, '_, '_, '_> {
    /// A span of the winnow parser's source, in the working set's span space.
    fn global(&self, span: nu_winnow_parser::Span) -> Span {
        Span::new(self.offset + span.start, self.offset + span.end)
    }

    /// Lower the next statement, `pipeline`, which parsing reported `diagnostics` in (see
    /// [`BlockSink::statement`]). Returns whether to go on with the next statement.
    fn take_statement(&mut self, pipeline: &w::Pipeline<'_>, diagnostics: &[Diagnostic]) -> bool {
        if !diagnostics.is_empty() {
            // A syntax error: the classic parser takes the rest of the block, reporting the
            // error as it always has.
            self.rest_to_classic(pipeline, true);
            return false;
        }
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let mut lower = Lower::new(working_set, self.source, self.offset);
        let span = lower.statement_span(pipeline);
        let mut target = statement_target(&mut self.target, &self.pipelines, self.span);
        lower.statement_or_classic(pipeline, span, &mut target, &mut self.pipelines);
        true
    }

    /// Hand the rest of the block, from `pipeline` on, to the classic parser. `had_errors`: the
    /// winnow parser reported an error in `pipeline`.
    fn rest_to_classic(&mut self, pipeline: &w::Pipeline<'_>, had_errors: bool) {
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let lower = Lower::new(working_set, self.source, self.offset);
        let start = lower.statement_span(pipeline).start;
        let mut target = statement_target(&mut self.target, &self.pipelines, self.span);
        // Where the statement ends is the classic parser's to decide, so it parses the rest
        // of the block.
        let span = Span::new(start, self.span.end);
        stats::record_classic_statement(span.len(), had_errors);
        parse_classic(
            lower.working_set,
            span,
            &mut target,
            true,
            &mut self.pipelines,
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
        Target::Block { input_type } => StatementTarget::Block {
            input_type: if pipelines.is_empty() {
                input_type.as_ref()
            } else {
                None
            },
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
        // The classic predeclaration, from the statement's text.
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
