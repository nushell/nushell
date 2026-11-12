//! Blocks, pipelines and pipeline elements (nu-parser's `parse_pipelines.rs`).

use crate::winnow::syntax::ast::{
    AttributeBlock, Block, BlockErrors, BlockLexError, Comment, Expr, Expression, LiteError,
    Pipeline, PipelineElement, PipelineRedirection, RedirectionTarget,
};
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseResult, cut, into_diagnostic};
use crate::winnow::syntax::lex::{
    LexError, LexOptions, RedirectionSource, Token, TokenContents, assignment_operator,
};
use crate::winnow::syntax::span::Span;

use super::WorkingSet;
use super::lite_parser::{LiteBlock, LiteBlockCommand, LiteCommand, lite_diagnostic};
use super::parse_calls::{keyword_signature_of_call, parse_attribute};
use super::parse_def::{
    Definitions, PredeclaredDef, TypeAliasPredecl, parse_def_predecl, read_predecls,
};
use super::parse_expressions::{
    ExpectedShape, Position, is_env_variable_name, parse_builtin_commands, parse_expression,
    parse_value,
};
use super::parse_helpers::garbage_pipeline;
use super::tokens::Tokens;
use super::working_set::{CommandLookup, ParseState};

/// The items after a binding's or an assignment's `=`, at `span`, as the block nu parses them
/// as (`let x = ls | length`): lexed again from their text as a block is when a newline is
/// between two of them, since in a subexpression, whose newlines were whitespace, a newline
/// ends a pipeline there.
pub fn parse_value_block<'a>(tokens: Tokens<'_, 'a>, span: Span) -> ParseResult<Block<'a>> {
    let working_set = tokens.working_set;
    let newline_between = tokens
        .all()
        .windows(2)
        .map(|pair| Span::new(pair[0].span.end, pair[1].span.start))
        .any(|gap| working_set.get_span_contents(gap).contains('\n'));
    if !newline_between {
        return Ok(parse_block(tokens, span));
    }
    let relexed = working_set
        .lex_block(span, LexOptions::BLOCK)
        .map_err(cut)?;
    Ok(parse_block(Tokens::from_lexed(working_set, &relexed), span))
}

/// The pipelines of a block covering `span` (nu's `parse_block`).
///
/// Errors are recorded in the working set and the offending statement becomes
/// an [`Expr::Garbage`] pipeline, so parsing continues with the next line.
/// [`BlockStatements`] runs the same steps a few statements at a time.
///
/// A block with a statement that changes which commands exist is left
/// unparsed, an empty block, when the lookup asks for that
/// ([`CommandLookup::defers_command_changing_blocks`]): one its first words tell, or a call
/// through an alias of an overlay command, found as it is parsed.
pub fn parse_block<'a>(tokens: Tokens<'_, 'a>, span: Span) -> Block<'a> {
    parse_block_lexed_from(tokens, span, span)
}

/// [`parse_block`] for a block whose text nu lexes as `lexed` (a closure's body, with its
/// parameters): the block's lexer error is the first in it.
pub fn parse_block_lexed_from<'a>(tokens: Tokens<'_, 'a>, span: Span, lexed: Span) -> Block<'a> {
    let working_set = tokens.working_set;
    let defers = working_set.defers_command_changing_blocks();
    let unparsed = Block {
        span,
        ..Block::default()
    };
    if defers && changes_commands(working_set, tokens.all()) {
        return unparsed;
    }
    let errors = working_set.error_count();
    let outer_overlay_alias_called = working_set.replace_overlay_alias_called(false);
    // nu reports the block's first lexer error, then a `|` that ends it, before its
    // definitions' and statements' errors, and goes on past both, as the consumer does, to
    // which the block's first and last pipelines carry them: they are recorded as recovered.
    let lex_error = working_set.first_lex_error_in(lexed);
    let lite_block = LiteBlock::parse(working_set.source, tokens.all());
    let missing_end = lite_block.missing_end();
    if let Some(error) = &lex_error {
        working_set.error(error.diagnostic().as_recovered());
    }
    if let Some(error) = missing_end {
        working_set.error(lite_diagnostic(error).as_recovered());
    }
    parse_def_predecl(working_set, &lite_block);
    // Every comment is the parse's, whichever command nu's lite parser keeps it with, if any.
    for token in tokens.all() {
        if token.contents == TokenContents::Comment {
            working_set.add_comment(token.span);
        }
    }
    let mut pipelines: Vec<Pipeline<'a>> = Vec::new();
    let mut next_command = 0;
    parse_statements(
        working_set,
        &lite_block,
        tokens.all(),
        &mut next_command,
        &mut pipelines,
    );
    // A call through an alias of an overlay command, which only the commands known as it is
    // parsed tell (`DeclKind::OverlayAlias`), changes the commands too: the block is left
    // unparsed after all, its errors dropped.
    if working_set.replace_overlay_alias_called(outer_overlay_alias_called) && defers {
        working_set.take_errors_from(errors);
        return unparsed;
    }
    // With no pipeline to carry them (`(|)`), the block carries its errors itself.
    let mut without_pipelines = BlockErrors::default();
    if let Some(error) = lex_error {
        let error = BlockLexError {
            error,
            lexed_from: lexed.start,
        };
        match pipelines.first_mut() {
            Some(first) => first.lex_error = Some(Box::new(error)),
            None => without_pipelines.lex_error = Some(error),
        }
    }
    if let Some(error) = missing_end {
        match pipelines.last_mut() {
            Some(last) => last.lite_error = Some(Box::new(error)),
            None => without_pipelines.lite_error = Some(error),
        }
    }
    // A dropped command's error (`e>|` alone), which no pipeline carries.
    if missing_end.is_none() {
        without_pipelines.lite_error = without_pipelines.lite_error.or(lite_block.dropped_error());
    }
    let without_pipelines =
        (without_pipelines != BlockErrors::default()).then(|| Box::new(without_pipelines));
    Block {
        span,
        pipelines,
        without_pipelines,
    }
}

/// Whether a statement of the block lexed as `tokens` may change which commands exist for the
/// statements after it, by the first words of its commands (after the start of the block, an
/// end of line, a `;` or a `|`): `use`, `module`, `export`, `overlay`, `hide`, `source`,
/// `source-env`, `run`, `plugin use`. One more is harmless: the block is only parsed again.
fn changes_commands(working_set: &WorkingSet<'_>, tokens: &[Token]) -> bool {
    let mut at_command = true;
    for (index, token) in tokens.iter().enumerate() {
        match token.contents {
            TokenContents::Eol | TokenContents::Semicolon | TokenContents::Pipe => {
                at_command = true
            }
            TokenContents::Comment => {}
            TokenContents::Item if at_command => {
                at_command = false;
                let next_word = || {
                    tokens
                        .get(index + 1)
                        .map(|next| working_set.get_span_contents(next.span))
                };
                match working_set.get_span_contents(token.span) {
                    "use" | "module" | "export" | "overlay" | "hide" | "source" | "source-env"
                    | "run" => return true,
                    "plugin" if next_word() == Some("use") => return true,
                    _ => {}
                }
            }
            _ => at_command = false,
        }
    }
    false
}

/// What [`crate::winnow::syntax::parse_block_streaming`] hands over, in source order: first every
/// definition of the block, then its statements one at a time.
pub trait BlockSink<'a> {
    /// A `def` or `extern` of the block, found before any statement is parsed
    /// (nu's `parse_def_predecl`), with its signature parsed.
    fn predecl(&mut self, def: PredeclaredDef<'a>);

    /// A `type` statement of the block, found before any statement is parsed (nu's
    /// `parse_def_predecl` records its name and span): an engine that expands type aliases
    /// records it, to tell a use above the definition from an unknown type.
    fn predecl_type_alias(&mut self, type_alias: TypeAliasPredecl) {
        let _ = type_alias;
    }

    /// The next statement of the block, parsed only after every earlier one
    /// was handed over, so that what an earlier statement declares (a `use`,
    /// an `alias`) is known to the [`CommandLookup`] while this one is parsed.
    /// `diagnostics` holds what parsing it reported; when it is not empty,
    /// `pipeline` may be a garbage placeholder that spans the statement.
    /// Returns whether to go on with the next statement.
    fn statement(&mut self, pipeline: Pipeline<'a>, diagnostics: Vec<Diagnostic>) -> bool;
}

/// A block's statements, parsed a few at a time (see [`crate::winnow::syntax::parse_block_streaming`]): the
/// block's tokens and how far parsing them got.
///
/// ```text
/// BlockStatements::new       lex the block, declare the names of its `def`s and `extern`s
/// Definitions::parse         their signatures, for the engine to declare before it takes a statement
/// BlockStatements::parse     statements, one by one, until the sink returns `false` ...
/// BlockStatements::parse     ... and on from there, until `is_done`
/// ```
///
/// A statement is parsed only after the previous one was handed over, so a [`CommandLookup`]
/// over the engine's live declarations knows what the earlier statements brought in (a `use`,
/// an `overlay use`). It borrows
/// nothing: the tokens are spans, what the parse knows between two calls is owned, and
/// `source` and `lookup` are passed to each call. So it can move to another thread between
/// two calls, and each call may resolve command names with a different lookup (a copy of the
/// engine's names, for a call on another thread).
#[derive(Debug)]
pub struct BlockStatements {
    /// The block's tokens, as the lexer returned them (`Eof` last).
    tokens: Vec<Token>,
    /// The tokens lite-parsed: the block's pipelines, its comments and its lite-parse errors.
    lite_block: LiteBlock,
    /// The next command to parse, the first of its pipeline.
    next_command: usize,
    /// The block's first lexer error, which nu goes on past.
    lex_error: Option<LexError>,
    /// What the working set knew when the last call returned: the block's scope, with the
    /// names `new` predeclared and the aliases parsed since, and the bracket groups measured.
    /// Each call builds its working set from it and takes it back.
    parse_state: ParseState,
}

impl BlockStatements {
    /// Lex the block covering `span` of `source` and declare the names of its definitions in
    /// the block's scope (nu's `parse_def_predecl`), so that a statement may call a command
    /// defined after it. Returns the statements still to parse and every `def`/`extern` of the
    /// block, whose signatures [`Definitions::parse`] parses: an engine declares them before it
    /// takes any statement, and the statements can be parsed meanwhile. Declaring the names
    /// asks `lookup` nothing.
    ///
    /// With `subexpression` the block is the inside of a `( ... )`, lexed as nu's
    /// `parse_subexpression` lexes it: newlines are whitespace and comments are skipped, so a
    /// statement goes on across lines (`(ls\n| length)`, `(echo a\nb)`).
    ///
    /// A diagnostic about the block as a whole that nu does not go on past is returned
    /// instead. The block's first lexer error ([`BlockStatements::lex_error`]), its lite-parse
    /// error ([`BlockStatements::lite_error`]) and a definition declared twice, which the
    /// consumer reports as it declares the definitions, nu goes on past.
    pub fn new<'a>(
        source: &'a str,
        span: Span,
        lookup: impl CommandLookup + 'a,
        subexpression: bool,
    ) -> Result<(Self, Definitions), Vec<Diagnostic>> {
        let working_set = WorkingSet::with_lookup(source, span, lookup);
        let options = if subexpression {
            LexOptions::SUBEXPRESSION
        } else {
            LexOptions::BLOCK
        };
        let mut tokens = working_set
            .lex_block(span, options)
            .map_err(|diagnostic| vec![diagnostic])?;
        let lex_error = working_set.first_lex_error_in(span);
        if subexpression {
            tokens.retain(|token| token.contents != TokenContents::Comment);
        }
        let stream = Tokens::from_lexed(&working_set, &tokens);
        let lite_block = LiteBlock::parse(source, stream.all());
        let definitions = read_predecls(&working_set, &lite_block);
        // What nu goes on past (a definition declared twice) the consumer reports.
        let block_errors = working_set.take_errors_from(0);
        if block_errors.iter().any(|diagnostic| !diagnostic.recovered) {
            return Err(block_errors);
        }
        let parse_state = working_set.into_state();
        Ok((
            Self {
                tokens,
                lite_block,
                next_command: 0,
                lex_error,
                parse_state,
            },
            definitions,
        ))
    }

    /// Parse the next statements, resolving command names with `lookup`, and hand each to
    /// `sink` with the diagnostics parsing it reported (when there are some, the pipeline may
    /// be a garbage placeholder that spans the statement) and the bound on command-name
    /// lengths it was parsed with: [`CommandLookup::longest_name`], raised for the names the
    /// parser declared itself; the statement looked up no longer name. A statement is parsed
    /// only after the previous one was handed over. Stops when `sink` returns `false` or the
    /// block ends.
    ///
    /// `source` is the text given to [`BlockStatements::new`]: the tokens are spans into it.
    pub fn parse<'a>(
        &mut self,
        source: &'a str,
        lookup: impl CommandLookup + 'a,
        sink: &mut dyn FnMut(Pipeline<'a>, Vec<Diagnostic>, usize) -> bool,
    ) {
        let parse_state = std::mem::take(&mut self.parse_state);
        let working_set = WorkingSet::with_state(source, lookup, parse_state);
        parse_statements(
            &working_set,
            &self.lite_block,
            &self.tokens,
            &mut self.next_command,
            &mut StreamingSink {
                sink,
                working_set: &working_set,
            },
        );
        self.parse_state = working_set.into_state();
    }

    /// Forget each command name the block's statements declared (its definitions, its aliases)
    /// that `lookup` answers otherwise for now, so that its answer counts from the next
    /// statement on: for a consumer that declares them where `lookup` looks, after a statement
    /// that changed which commands exist there (a `hide`, an `overlay hide`, a `use` that binds
    /// a name again). The parser's own declarations shadow the lookup's answers otherwise.
    pub fn forget_changed_names(&mut self, lookup: impl CommandLookup) {
        self.parse_state.forget_names_unlike(&lookup);
    }

    /// The block's lite-parse error, which nu reports before any statement's error, having
    /// lite-parsed the whole block first: a `|` that ends the block
    /// ([`LiteError::PipelineMissingEnd`]), else its first command's.
    pub fn lite_error(&self) -> Option<LiteError> {
        self.lite_block.error()
    }

    /// The block's first lexer error, which nu reports before its statements' errors and goes
    /// on past; the text it is in starts where the block does.
    pub fn lex_error(&self) -> Option<&LexError> {
        self.lex_error.as_ref()
    }

    /// Whether every statement was parsed.
    pub fn is_done(&self) -> bool {
        self.next_command >= self.lite_block.command_count()
    }

    /// The text of the statements still to parse: from the next command to the end of the block.
    pub fn remaining(&self) -> Span {
        let end = self.tokens.last().map_or(0, |token| token.span.end);
        let start = self
            .lite_block
            .command_at(self.next_command)
            .and_then(|command| self.lite_block.parts(command).first())
            .map_or(end, |first| first.span.start);
        Span::new(start, end)
    }

    /// The first two words of the next statement (the second when it is on the same line),
    /// to tell what kind of statement comes without parsing it.
    pub fn next_words<'a>(&self, source: &'a str) -> (Option<&'a str>, Option<&'a str>) {
        let parts = self
            .lite_block
            .command_at(self.next_command)
            .map_or(&[][..], |command| self.lite_block.parts(command));
        let mut words = parts
            .iter()
            .map_while(|token| token.is_item().then(|| token.text(source)));
        (words.next(), words.next())
    }
}

/// Where [`parse_statements`] puts the pipelines it parses: a `Vec` for [`parse_block`],
/// which leaves every diagnostic in the working set for the whole parse to collect, or a
/// [`StreamingSink`] for [`BlockStatements::parse`], which hands each statement over with
/// its own diagnostics.
trait StatementSink<'a> {
    /// Take the next pipeline. `before` is how the working set stood just before it was parsed.
    /// Returns whether to go on.
    fn push(&mut self, pipeline: Pipeline<'a>, before: StatementStart) -> bool;
    /// Take the placeholder for a statement that failed to parse. Returns whether to go on.
    fn push_garbage(&mut self, garbage: Pipeline<'a>, before: StatementStart) -> bool {
        self.push(garbage, before)
    }
}

/// The whole block at once ([`parse_block`]): the diagnostics stay in the working set.
impl<'a> StatementSink<'a> for Vec<Pipeline<'a>> {
    fn push(&mut self, pipeline: Pipeline<'a>, _before: StatementStart) -> bool {
        Vec::push(self, pipeline);
        true
    }
}

/// How the working set stood just before a statement was parsed.
#[derive(Clone, Copy)]
struct StatementStart {
    /// [`WorkingSet::error_count`]: every diagnostic recorded since, those of the statement's
    /// nested blocks included, is the statement's.
    errors: usize,
    /// [`WorkingSet::longest_decl_name`]: the statement looked up no longer name.
    longest_name: usize,
}

/// Hands each pipeline to a [`BlockStatements::parse`] sink with the diagnostics parsing it
/// reported, taken out of the working set.
struct StreamingSink<'s, 'w, 'a> {
    /// The caller's sink: takes a statement and says whether to go on.
    sink: &'s mut dyn FnMut(Pipeline<'a>, Vec<Diagnostic>, usize) -> bool,
    /// The working set the statements are parsed in, where their diagnostics are recorded.
    working_set: &'w WorkingSet<'a>,
}

impl<'a> StatementSink<'a> for StreamingSink<'_, '_, 'a> {
    fn push(&mut self, pipeline: Pipeline<'a>, before: StatementStart) -> bool {
        let diagnostics = self.working_set.take_errors_from(before.errors);
        (self.sink)(pipeline, diagnostics, before.longest_name)
    }
}

/// The statements of a block from the command `next_command` on, each a pipeline of `lite_block`,
/// pushed to `out` as they are parsed, until `out` says to stop. `next_command` is up to date before
/// each push, so that a later call goes on from there.
fn parse_statements<'a>(
    working_set: &WorkingSet<'a>,
    lite_block: &LiteBlock,
    tokens: &[Token],
    next_command: &mut usize,
    out: &mut dyn StatementSink<'a>,
) {
    for commands in lite_block.pipelines_from(*next_command) {
        let first_command = *next_command;
        *next_command += commands.len();
        let before = StatementStart {
            errors: working_set.error_count(),
            longest_name: working_set.longest_decl_name(),
        };
        // nu reports these and goes on, and so does the consumer: recorded as recovered.
        let mut lite_error = None;
        for error in lite_block.errors_in(first_command, *next_command) {
            working_set.error(lite_diagnostic(error).as_recovered());
            lite_error.get_or_insert(error);
        }
        let parsed = parse_pipeline(working_set, lite_block, tokens, commands, lite_error);
        let go_on = match parsed {
            Ok(pipeline) => out.push(pipeline, before),
            Err(error) => {
                // An error the statement failed on was gone past by nothing, even one marked
                // recovered where it was made, which only the construct's own parser goes past:
                // the statement is to be parsed again.
                let mut diagnostic = into_diagnostic(error);
                diagnostic.recovered = false;
                working_set.error(diagnostic);
                let garbage = garbage_statement(lite_block, tokens, commands, lite_error);
                out.push_garbage(garbage, before)
            }
        };
        if !go_on {
            return;
        }
    }
    // A command nu dropped after the last one (`ls |`, then `e>|`) has no pipeline to report it.
    for error in lite_block.errors_from(lite_block.command_count()) {
        working_set.error(lite_diagnostic(error).as_recovered());
    }
}

/// The placeholder for the statement of `commands`, which failed to parse: from the first of its
/// tokens to the last the lite parse gave it, comments on its lines and a `|` that ends it included.
/// Its comments go with it, for whoever parses it again.
fn garbage_statement<'a>(
    lite_block: &LiteBlock,
    tokens: &[Token],
    commands: &[LiteBlockCommand],
    lite_error: Option<LiteError>,
) -> Pipeline<'a> {
    let (leading_comments, _) = pipeline_comments(lite_block, commands);
    let first = commands
        .first()
        .and_then(|command| {
            command
                .pipe_before
                .or_else(|| lite_block.parts(command).first().map(|first| first.span))
        })
        .unwrap_or_default();
    let last = commands.last();
    let end = last.and_then(|last| {
        let ended_at = last.ended_at as usize;
        match tokens.get(ended_at) {
            Some(token) if is_pipe(token) => Some(token.span),
            _ => tokens.get(ended_at.checked_sub(1)?).map(|token| token.span),
        }
    });
    let mut garbage = garbage_pipeline(end.map_or(first, |end| first.merge(end)));
    garbage.leading_comments = leading_comments;
    garbage.lite_error = lite_error.map(Box::new);
    garbage.terminator = last.and_then(|last| terminator(tokens, last));
    garbage
}

/// A `|`, or a redirection into one (`e>|`).
fn is_pipe(token: &Token) -> bool {
    match token.contents {
        TokenContents::Pipe => true,
        TokenContents::Redirection(operator) => operator.is_pipe(),
        _ => false,
    }
}

/// The `;` that ends the pipeline whose last command is `last`, right after it or after a pipe that
/// no command follows.
fn terminator(tokens: &[Token], last: &LiteBlockCommand) -> Option<Span> {
    tokens
        .get(last.ended_at as usize..)?
        .iter()
        .find(|token| !is_pipe(token) && token.contents != TokenContents::Comment)
        .filter(|token| token.contents == TokenContents::Semicolon)
        .map(|token| token.span)
}

/// The comments nu's lite parser keeps with the commands of a pipeline: those of its first command
/// before the command's first part, which are the statement's own, and the rest.
fn pipeline_comments(
    lite_block: &LiteBlock,
    commands: &[LiteBlockCommand],
) -> (Vec<Comment>, Vec<Comment>) {
    let mut leading = Vec::new();
    let mut trailing = Vec::new();
    let first_part = commands
        .first()
        .and_then(|first| lite_block.parts(first).first())
        .map_or(0, |first| first.span.start);
    for command in commands {
        for span in lite_block.comments(command) {
            let comment = Comment { span: *span };
            if span.start < first_part {
                leading.push(comment);
            } else {
                trailing.push(comment);
            }
        }
    }
    (leading, trailing)
}

/// One pipeline (nu's `parse_pipeline`), `commands` of `lite_block`, with `lite_error` the first
/// lite-parse error among them.
fn parse_pipeline<'a>(
    working_set: &WorkingSet<'a>,
    lite_block: &LiteBlock,
    tokens: &[Token],
    commands: &[LiteBlockCommand],
    lite_error: Option<LiteError>,
) -> ParseResult<Pipeline<'a>> {
    let single = commands.len() == 1;
    if !single {
        for command in commands {
            check_loop_in_pipeline(working_set, &lite_block.command(command, tokens));
        }
    }
    let mut elements: Vec<PipelineElement<'a>> = Vec::with_capacity(commands.len());
    let mut previous: Option<&LiteBlockCommand> = None;
    for command in commands {
        let lite_command = lite_block.command(command, tokens);
        let redirection_spans = lite_command
            .redirections
            .iter()
            .flat_map(|(operator, target)| {
                std::iter::once(operator.span).chain(target.map(|target| target.span))
            })
            .collect();
        let outer = working_set.replace_element_redirections(redirection_spans);
        let parsed = parse_pipeline_element(working_set, &lite_command, !single);
        working_set.replace_element_redirections(outer);
        let (expr, redirection) = parsed?;
        // The last `|` before it, which the command before took if no command came between.
        let pipe = command
            .pipe_before
            .or_else(|| previous.and_then(|previous| previous.pipe));
        let start = pipe.map_or(expr.span.start, |pipe| pipe.start);
        let end = redirection.as_ref().map_or(expr.span.end, |redirection| {
            redirection.span().end.max(expr.span.end)
        });
        // From the first attribute item, or else the first part, to the last part.
        let all_parts = lite_block.parts(command);
        let parts = match (all_parts.first(), all_parts.last()) {
            (Some(first), Some(last)) => first.span.merge(last.span),
            _ => expr.span,
        };
        elements.push(PipelineElement {
            span: Span::new(start, end),
            pipe,
            lite_pipe: command.pipe,
            expr,
            redirection,
            parts,
        });
        previous = Some(command);
    }
    let span = elements[0].span.merge(
        elements
            .last()
            .map_or(elements[0].span, |element| element.span),
    );
    let last = commands.last();
    let (leading_comments, trailing_comments) = pipeline_comments(lite_block, commands);
    Ok(Pipeline {
        span,
        elements,
        leading_comments,
        trailing_comments,
        terminator: last.and_then(|last| terminator(tokens, last)),
        // A `|` after the last command, which no command follows.
        dangling_pipe: last
            .and_then(|last| tokens.get(last.ended_at as usize))
            .filter(|token| token.contents == TokenContents::Pipe)
            .map(|token| token.span),
        lite_error: lite_error.map(Box::new),
        lex_error: None,
    })
}

/// Report a `while` or `loop` that is one command of a longer pipeline (nu's `parse_pipeline`):
/// loops are statements without output, so like `for` (refused in `parse_expression`) they
/// can't be piped. The head is the first item after any `NAME=value` environment shorthands, as
/// there, and an attribute line's first item counts as one. nu reports each such command
/// before it parses the pipeline, which it then parses as any other, and so does the consumer:
/// the error is recorded as recovered.
fn check_loop_in_pipeline(working_set: &WorkingSet<'_>, lite_command: &LiteCommand<'_>) {
    let is_env_shorthand = |token: &&Token| {
        let text = working_set.get_span_contents(token.span);
        text.find('=')
            .is_some_and(|equals| is_env_variable_name(&text[..equals]))
    };
    let head = lite_command
        .attributes
        .iter()
        .flat_map(|line| line.iter())
        .chain(lite_command.parts)
        .find(|token| !is_env_shorthand(token));
    if let Some((head, keyword @ ("while" | "loop"))) =
        head.map(|head| (head, working_set.get_span_contents(head.span)))
    {
        working_set.error(
            Diagnostic::new(ErrorKind::KeywordInPipeline(keyword.to_string()), head.span)
                .with_help("a loop has no output to pipe; collect values with `each`, `reduce` or a `mut` variable")
                .as_recovered(),
        );
    }
}

/// One command of a pipeline, as collected by the lite parse (nu's
/// `parse_pipeline_element`). `in_pipeline` is set when it is one of several
/// elements; a lone command may be a statement (nu's `parse_builtin_commands`).
fn parse_pipeline_element<'a>(
    working_set: &WorkingSet<'a>,
    lite_command: &LiteCommand<'_>,
    in_pipeline: bool,
) -> ParseResult<(Expression<'a>, Option<PipelineRedirection<'a>>)> {
    let position = if in_pipeline {
        Position::Element
    } else {
        Position::Statement
    };
    let expr = match lite_command.attributes.as_slice() {
        // A lone `let` or `mut` reads its redirection as its own items (nu's `parse_let` and
        // `parse_mut` take `parts_including_redirection`), so `let a o> f = 1` has extra items
        // before its `=` and `let x o> f` a word where its `=` goes: the statement is that
        // binding or call, with no redirection.
        [] if !in_pipeline
            && !lite_command.redirections.is_empty()
            && lite_command.parts.first().is_some_and(|head| {
                matches!(working_set.get_span_contents(head.span), "let" | "mut")
            }) =>
        {
            let mut items = lite_command.parts.to_vec();
            for (operator, target) in lite_command.redirections {
                items.push(Token {
                    contents: TokenContents::Item,
                    span: operator.span,
                });
                items.extend(*target);
            }
            items.sort_unstable_by_key(|token| token.span.start);
            let end = items.last().map_or(lite_command.end, |last| last.span.end);
            let expr = parse_expression(Tokens::new(working_set, &items, end), position)?;
            return Ok((expr, None));
        }
        [] => parse_expression(lite_command.tokens(working_set), position)?,
        // nu parses an element of a longer pipeline from all its items, the
        // attribute lines included, so the first attribute item is the head:
        // `@e\nx | y` calls an unknown `@e`, and `@e; def f [] {} | y` is no
        // statement in a pipeline.
        attribute_lines if in_pipeline => {
            // nu finds an assignment by the text of the items, so an `=` of an
            // attribute line counts (`@a x = y\nb | c` assigns to `@a x`).
            let items: Vec<Token> = attribute_lines
                .iter()
                .flat_map(|line| line.iter())
                .map(
                    |token| match assignment_operator(working_set.get_span_contents(token.span)) {
                        Some(operator) => Token {
                            contents: TokenContents::AssignmentOperator(operator),
                            span: token.span,
                        },
                        None => *token,
                    },
                )
                .chain(lite_command.parts.iter().copied())
                .collect();
            parse_expression(Tokens::new(working_set, &items, lite_command.end), position)?
        }
        attribute_lines => {
            let attributes = attribute_lines
                .iter()
                .map(|attribute_line| parse_attribute(working_set, attribute_line))
                .collect::<ParseResult<Vec<_>>>()?;
            let words: Vec<&str> = lite_command
                .parts
                .iter()
                .take(2)
                .map(|token| working_set.get_span_contents(token.span))
                .collect();
            let is_definition = matches!(
                words.as_slice(),
                ["def" | "extern", ..] | ["export", "def" | "extern"]
            );
            let item = match (is_definition, lite_command.parts.first()) {
                (true, Some(first)) if in_pipeline => {
                    return Err(cut(Diagnostic::new(
                        ErrorKind::KeywordInPipeline(
                            working_set.get_span_contents(first.span).to_string(),
                        ),
                        first.span,
                    )));
                }
                (true, _) => parse_builtin_commands(lite_command.tokens(working_set))?,
                // Before anything else nu reports the attributes (`parse_attribute_block`) and
                // keeps the command, or the point after the last attribute, as garbage: so does
                // the parser, the error recorded as recovered. (An `export` nu reads with its
                // attributes as an export, `parse_export_in_block`, which does the same.)
                (false, Some(first)) => {
                    working_set.error(
                        Diagnostic::message(
                            "attributes must be followed by a definition",
                            first.span,
                        )
                        .with_help(
                            "only `def`, `extern`, `export def` and `export extern` take attributes",
                        )
                        .as_recovered(),
                    );
                    let last = lite_command.parts.last().unwrap_or(first);
                    Expression::new(Expr::Garbage, first.span.merge(last.span))
                }
                (false, None) => {
                    let last = attributes
                        .last()
                        .map_or(Span::point(lite_command.end), |attribute| attribute.span);
                    working_set.error(
                        Diagnostic::message(
                            "attributes must be followed by a definition",
                            last.past(),
                        )
                        .with_help("put a `def` or `extern` on the line after the attributes")
                        .as_recovered(),
                    );
                    Expression::new(Expr::Garbage, last.past())
                }
            };
            let span = attributes[0].span.merge(item.span);
            Expression::new(
                Expr::AttributeBlock(AttributeBlock {
                    attributes,
                    item: Box::new(item),
                }),
                span,
            )
        }
    };
    // nu's command covers all of its items, those it ignores included (`try {} --`).
    let mut expr = expr;
    if let Some(last) = lite_command.parts.last() {
        expr.span.end = expr.span.end.max(last.span.end);
    }
    let redirection = parse_redirection(working_set, lite_command)?;
    if redirection.is_some() && drops_redirection(working_set, &expr, in_pipeline) {
        // nu never looks at a redirection on `export-env`, a `const` without a value (or one
        // asking for help) or attributes before no definition: it is ignored text, not
        // refused, so this comes before `rejects_redirection`.
        for (operator, target) in lite_command.redirections {
            working_set.add_ignored(operator.span);
            if let Some(target) = target {
                working_set.add_ignored(target.span);
            }
        }
        return Ok((expr, None));
    }
    if redirection.is_some() && rejects_redirection(&expr, in_pipeline) {
        let at = lite_command
            .redirections
            .first()
            .map_or(expr.span, |(operator, _)| operator.span);
        let diagnostic = Diagnostic::message("this statement cannot be redirected", at);
        // nu reports the statement and makes it garbage (`redirecting_builtin_error`), and so
        // does the consumer: the error is recorded as recovered. Before a definition nu reads
        // the attributes first and keeps them; an `export` it refuses before reading them,
        // except in a module's body, where it reads them and drops the redirection: the
        // consumer tells those apart.
        working_set.error(diagnostic.as_recovered());
    }
    Ok((expr, redirection))
}

/// The statements whose redirection nu ignores: `export-env` (`parse_export_env` reads only
/// its first item), its call included (`export-env {a: 1} o> f`), a `const` without a
/// value, whose call `parse_const` reads from the command's parts alone (with a value, the
/// redirection is part of it), and attributes before a command that is no definition and no
/// `export` (`parse_attribute_block` keeps the command's parts as garbage). In a longer
/// pipeline (`in_pipeline`) a `const` is an ordinary call, redirected as any
/// (`const x e>| length`).
fn drops_redirection(
    working_set: &WorkingSet<'_>,
    expr: &Expression<'_>,
    in_pipeline: bool,
) -> bool {
    if in_pipeline {
        return false;
    }
    match &expr.expr {
        Expr::ExportEnv(_) | Expr::Const(_) => true,
        Expr::Call(call) => matches!(call.head.name.as_ref(), "const" | "export-env"),
        Expr::AttributeBlock(block) => {
            let first_word = working_set
                .get_span_contents(block.item.span)
                .split_whitespace()
                .next();
            matches!(block.item.expr, Expr::Garbage) && first_word != Some("export")
        }
        _ => false,
    }
}

/// The statements nu refuses to redirect (nu's `redirecting_builtin_error`) when they are the
/// whole statement (`in_pipeline` false). A lone `let` or `mut` never gets here: it reads the
/// redirection as its own items ([`parse_pipeline_element`]). In a longer pipeline `let` is
/// an ordinary call, redirected as any (`let x e>| length`, `[1] | let x o> f`).
fn rejects_redirection(expr: &Expression<'_>, in_pipeline: bool) -> bool {
    // nu refuses the redirection in a statement's parse function, which reads a pipeline of
    // one command (`parse_builtin_commands`); in a longer one each command is read where a
    // command is (`parse_expression`), which refuses none (`run null e>| ls`).
    if in_pipeline {
        return false;
    }
    match &expr.expr {
        Expr::Def(_)
        | Expr::Extern(_)
        | Expr::For(_)
        | Expr::Alias(_)
        | Expr::TypeAlias(_)
        | Expr::Module(_)
        | Expr::Use(_)
        | Expr::Export(_)
        | Expr::AttributeBlock(_) => true,
        // `overlay <anything>` is refused by name before its arguments are looked at, and so
        // is a statement keyword read as a call (`def --help`, `module x --foo {}`): its parse
        // function refuses the redirection before it looks for help.
        Expr::Call(call) => {
            call.head.name.split(' ').next() == Some("overlay")
                || matches!(
                    call.head.name.as_ref(),
                    "def" | "extern" | "for" | "alias" | "type" | "module" | "use" | "export"
                )
                || keyword_signature_of_call(call).is_some()
        }
        _ => false,
    }
}

/// The redirections of a command (nu's `parse_redirection`): one stream
/// (`o> f`, `o+e> f`, `e>| cmd`), or stdout and stderr separately (`o> f e> g`,
/// in either order). Redirecting the same stream twice is an error.
fn parse_redirection<'a>(
    working_set: &WorkingSet<'a>,
    lite_command: &LiteCommand<'_>,
) -> ParseResult<Option<PipelineRedirection<'a>>> {
    let mut redirection: Option<PipelineRedirection<'a>> = None;
    for (operator, target) in lite_command.redirections {
        let target = match target {
            Some(token) => RedirectionTarget::File {
                op: *operator,
                append: operator.item.is_append(),
                path: Box::new(parse_value(working_set, token.span, ExpectedShape::Any)?),
            },
            None => RedirectionTarget::Pipe { op: *operator },
        };
        redirection = Some(match (redirection.take(), operator.item.source()) {
            (None, source) => PipelineRedirection::Single { source, target },
            (
                Some(PipelineRedirection::Single {
                    source: RedirectionSource::Stdout,
                    target: stdout,
                }),
                RedirectionSource::Stderr,
            ) => PipelineRedirection::Separate {
                out: stdout,
                err: target,
            },
            (
                Some(PipelineRedirection::Single {
                    source: RedirectionSource::Stderr,
                    target: stderr,
                }),
                RedirectionSource::Stdout,
            ) => PipelineRedirection::Separate {
                out: target,
                err: stderr,
            },
            (Some(previous), _) => {
                return Err(cut(Diagnostic::message(
                    "multiple redirections of the same stream",
                    operator.span,
                )
                .with_help(format!(
                    "the stream is already redirected at {}",
                    previous.span()
                ))));
            }
        });
    }
    Ok(redirection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winnow::syntax::parser::{DeclKind, ParseConfig};

    /// The pipelines of `source`, which must parse.
    fn parse(source: &str) -> Vec<Pipeline<'_>> {
        let (ast, diagnostics) = crate::winnow::syntax::parser::parse(source, &ParseConfig::new());
        assert!(diagnostics.is_empty(), "{source:?}: {diagnostics:?}");
        ast.block.pipelines
    }

    /// The number of elements of each pipeline of `source`.
    fn pipelines(source: &str) -> Vec<usize> {
        parse(source)
            .iter()
            .map(|pipeline| pipeline.elements.len())
            .collect()
    }

    #[test]
    fn line_leading_pipe_after_one_blank_line_continues() {
        assert_eq!(pipelines("[1 2 3] |\n\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] |\n# c\n\n# d\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] | # c\n\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] |\n\n\n| length"), [1, 1]);
        assert_eq!(pipelines("[1 2 3]\n\n| length"), [1, 1]);
    }

    /// Where the pipe nu's lite parser keeps with each command of `source` starts.
    fn lite_pipes(source: &str) -> Vec<Option<usize>> {
        // Some of these have lite-parse errors, which nu goes on past.
        let (ast, _) = crate::winnow::syntax::parser::parse(source, &ParseConfig::new());
        ast.block
            .pipelines
            .iter()
            .flat_map(|pipeline| &pipeline.elements)
            .map(|element| element.lite_pipe.map(|pipe| pipe.start))
            .collect()
    }

    #[test]
    fn a_command_keeps_the_pipe_that_ended_it_or_the_last_no_command_took() {
        assert_eq!(lite_pipes("ls | lines"), [Some(3), None]);
        assert_eq!(lite_pipes("| ls"), [Some(0)]);
        assert_eq!(lite_pipes("ls | | lines"), [Some(3), Some(5)]);
        assert_eq!(lite_pipes("ls | e>| |"), [Some(3)]);
        assert_eq!(lite_pipes("ls | | e>|"), [Some(3)]);
        assert_eq!(lite_pipes("ls e>| lines"), [Some(3), None]);
        assert_eq!(lite_pipes("ls e> f e>|"), [Some(8)]);
        assert_eq!(lite_pipes("^ls e>||\n\nlines"), [Some(4), Some(7)]);
        assert_eq!(lite_pipes("ls | |\n\nlines"), [Some(3), Some(5)]);
        assert_eq!(lite_pipes("^ls o>\n\n| lines"), [None, Some(8)]);
        assert_eq!(lite_pipes("[1 2 3] |\n\n| length"), [Some(8), Some(11)]);
        assert_eq!(lite_pipes("[1 2 3]\n\n| length"), [None, Some(9)]);
    }

    #[test]
    fn redirection_pipe_continues_only_through_a_line_leading_pipe() {
        assert_eq!(pipelines("^ls e>|\nlines"), [1, 1]);
        assert_eq!(pipelines("^ls o+e>| # c\nlines"), [1, 1]);
        assert_eq!(pipelines("^ls e>|\n# c\nlines"), [1, 1]);
        assert_eq!(pipelines("^ls e>|\n# c\n| lines"), [2]);
        assert_eq!(pipelines("^ls e>|\n\n| lines"), [1, 1]);
        assert_eq!(pipelines("^ls e>| lines"), [2]);
    }

    /// The attributes of the one statement of `source`, an attributed definition, by their
    /// number of arguments.
    fn attribute_arguments(source: &str) -> Vec<usize> {
        let pipelines = parse(source);
        let [Pipeline { elements, .. }] = &pipelines[..] else {
            panic!("{source:?}: {pipelines:?}")
        };
        let [
            PipelineElement {
                expr:
                    Expression {
                        expr: Expr::AttributeBlock(block),
                        ..
                    },
                ..
            },
        ] = &elements[..]
        else {
            panic!("{source:?}: {elements:?}")
        };
        block
            .attributes
            .iter()
            .map(|attribute| attribute.arguments.len())
            .collect()
    }

    #[test]
    fn attribute_lines_where_a_line_starts() {
        // A command that starts a line may have attribute lines, after a `|` too.
        assert_eq!(
            attribute_arguments("|\n@search-terms foo\ndef bar [] {}"),
            [1]
        );
        assert_eq!(
            pipelines("ls | length |\n@search-terms foo\ndef bar [] {}"),
            [3]
        );
        // A `|` that starts the next line goes on with the attribute line.
        assert_eq!(
            attribute_arguments("@search-terms foo\n| bar\ndef baz [] {}"),
            [3]
        );
    }

    /// No commands at all: every head is an external command.
    struct NoCommands;

    impl CommandLookup for NoCommands {
        fn find_decl(&self, _: &str) -> Option<DeclKind> {
            None
        }

        fn is_decl_name_prefix(&self, _: &str) -> bool {
            false
        }

        fn find_builtin_decl(&self, _: &str) -> Option<DeclKind> {
            None
        }
    }

    /// The text of each statement of the block covering `span` of `source`.
    fn statements(source: &str, span: Span, subexpression: bool) -> Vec<&str> {
        let (mut statements, _) =
            BlockStatements::new(source, span, NoCommands, subexpression).unwrap();
        let mut texts = Vec::new();
        statements.parse(source, NoCommands, &mut |pipeline, diagnostics, _| {
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            texts.push(pipeline.span.slice(source));
            true
        });
        texts
    }

    #[test]
    fn subexpression_statements_go_on_across_lines() {
        // As nu's `parse_subexpression` lexes them: newlines are whitespace, comments skipped.
        let source = "(echo a\n  b # c\n  | length)";
        let inner = Span::new(1, source.len() - 1);
        assert_eq!(
            statements(source, inner, true),
            ["echo a\n  b # c\n  | length"]
        );
        assert_eq!(
            statements(source, inner, false),
            ["echo a", "b # c\n  | length"]
        );
    }
}
