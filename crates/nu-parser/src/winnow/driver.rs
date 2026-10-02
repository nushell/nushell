//! Parsing a block statement by statement with the winnow parser.

use std::cell::RefCell;

use nu_protocol::{
    Module, Span, Type,
    ast::{Block, Pipeline},
    engine::StateWorkingSet,
};
use nu_winnow_parser::{BlockSink, Diagnostic, PredeclaredDef, ast as w};

use super::{
    lookup::EngineLookup,
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
    let result = nu_winnow_parser::parse_block_streaming(source, local_span, &lookup, &mut driver);
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
        let cell = self.working_set;
        let working_set = &mut **cell.borrow_mut();
        let mut lower = Lower::new(working_set, self.source, self.offset);
        let span = lower.statement_span(&pipeline);
        let mut target = statement_target(&mut self.target, &self.pipelines, self.span);
        if !diagnostics.is_empty() {
            // A syntax error: where the statement ends is the classic parser's to decide, so
            // it parses the rest of the block, reporting the error as it always has.
            let span = Span::new(span.start, self.span.end);
            stats::record_classic_statement(span.len(), true);
            let pipelines = parse_classic(lower.working_set, span, &mut target, true);
            self.pipelines.extend(pipelines);
            return false;
        }
        let pipelines = lower.statement_or_classic(&pipeline, span, &mut target);
        self.pipelines.extend(pipelines);
        true
    }
}
