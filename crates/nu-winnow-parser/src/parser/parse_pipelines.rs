//! Blocks, pipelines and pipeline elements (nu-parser's `parse_pipelines.rs`).

use crate::ast::{
    AttributeBlock, Block, Comment, Expr, Expression, Pipeline, PipelineElement, PipelineRedirection, RedirectionTarget,
};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut, into_diagnostic};
use crate::lex::{LexOptions, RedirectionSource, Token, TokenContents, assignment_operator};
use crate::span::Span;

use super::WorkingSet;
use super::lite_parser::{
    AfterPipe, LiteCommand, after_pipe, last_non_comment_token, parse_lite_command, skip_to_statement_end,
    take_pipe_on_later_line,
};
use super::parse_calls::{keyword_signature_of_call, parse_attribute};
use super::parse_def::{Definitions, PredeclaredDef, parse_def_predecl, scan_predecls};
use super::parse_expressions::{ExpectedShape, Position, parse_builtin_commands, parse_expression, parse_value};
use super::parse_helpers::garbage_pipeline;
use super::tokens::Tokens;
use super::working_set::{CommandLookup, ParseState};

/// The pipelines of a block covering `span` (nu's `parse_block`).
///
/// Errors are recorded in the working set and the offending statement becomes
/// an [`Expr::Garbage`] pipeline, so parsing continues with the next line.
/// [`BlockStatements`] runs the same steps a few statements at a time.
pub fn parse_block<'a>(mut tokens: Tokens<'_, 'a>, span: Span) -> Block<'a> {
    let working_set = tokens.working_set;
    parse_def_predecl(working_set, tokens.all());
    check_dangling_pipe(working_set, tokens.all());
    let mut pipelines: Vec<Pipeline<'a>> = Vec::new();
    parse_statements(&mut tokens, &mut StatementsState::default(), &mut pipelines);
    Block { span, pipelines }
}

/// What [`crate::parse_block_streaming`] hands over, in source order: first every
/// definition of the block, then its statements one at a time.
pub trait BlockSink<'a> {
    /// A `def` or `extern` of the block, found before any statement is parsed
    /// (nu's `parse_def_predecl`), with its signature parsed.
    fn predecl(&mut self, def: PredeclaredDef<'a>);

    /// The next statement of the block, parsed only after every earlier one
    /// was handed over, so that what an earlier statement declares (a `use`,
    /// an `alias`) is known to the [`CommandLookup`] while this one is parsed.
    /// `diagnostics` holds what parsing it reported; when it is not empty,
    /// `pipeline` may be a garbage placeholder that spans the statement.
    /// Returns whether to go on with the next statement.
    fn statement(&mut self, pipeline: Pipeline<'a>, diagnostics: Vec<Diagnostic>) -> bool;
}

/// A block's statements, parsed a few at a time (see [`crate::parse_block_streaming`]): the
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
    /// The next token to parse.
    position: usize,
    /// Where the statement loop stood when the last call returned: the comments waiting for
    /// the next statement, a `|` carried over to it.
    statements: StatementsState,
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
    /// A lexing error, or a diagnostic about the block as a whole (a definition declared
    /// twice, a `|` that ends the block), is returned instead.
    pub fn new<'a>(
        source: &'a str,
        span: Span,
        lookup: impl CommandLookup + 'a,
        subexpression: bool,
    ) -> Result<(Self, Definitions), Vec<Diagnostic>> {
        let working_set = WorkingSet::with_lookup(source, span, lookup);
        let options = if subexpression { LexOptions::SUBEXPRESSION } else { LexOptions::BLOCK };
        let mut tokens = working_set.lex(span, options).map_err(|diagnostic| vec![diagnostic])?;
        if subexpression {
            tokens.retain(|token| token.contents != TokenContents::Comment);
        }
        let stream = Tokens::from_lexed(&working_set, &tokens);
        let definitions = scan_predecls(&working_set, stream.all());
        check_dangling_pipe(&working_set, stream.all());
        let block_errors = working_set.take_errors_from(0);
        if !block_errors.is_empty() {
            return Err(block_errors);
        }
        let parse_state = working_set.into_state();
        Ok((Self { tokens, position: 0, statements: StatementsState::default(), parse_state }, definitions))
    }

    /// Parse the next statements, resolving command names with `lookup`, and hand each to
    /// `sink` with the diagnostics parsing it reported (when there are some, the pipeline may
    /// be a garbage placeholder that spans the statement). A statement is parsed only after
    /// the previous one was handed over. Stops when `sink` returns `false` or the block ends.
    ///
    /// `source` is the text given to [`BlockStatements::new`]: the tokens are spans into it.
    /// A statement that fails to parse is handed over without the `;` after it and a comment
    /// after that `;`: `parse_block` adds them to its garbage pipeline once it meets them, and
    /// a streamed pipeline is the sink's by then.
    pub fn parse<'a>(
        &mut self,
        source: &'a str,
        lookup: impl CommandLookup + 'a,
        sink: &mut dyn FnMut(Pipeline<'a>, Vec<Diagnostic>) -> bool,
    ) {
        let parse_state = std::mem::take(&mut self.parse_state);
        let working_set = WorkingSet::with_state(source, lookup, parse_state);
        let mut tokens = Tokens::from_lexed(&working_set, &self.tokens);
        tokens.reset_to(self.position);
        parse_statements(&mut tokens, &mut self.statements, &mut StreamingSink { sink, working_set: &working_set });
        self.position = tokens.position();
        self.parse_state = working_set.into_state();
    }

    /// Whether every statement was parsed.
    pub fn is_done(&self) -> bool {
        self.position >= self.stream_len()
    }

    /// The text of the statements still to parse: from the next token to the end of the block.
    pub fn remaining(&self) -> Span {
        let end = self.tokens.last().map_or(0, |token| token.span.end);
        let start = self.tokens.get(self.position).map_or(end, |token| token.span.start);
        Span::new(start, end)
    }

    /// The first two words of the next statement (the second when it is on the same line),
    /// to tell what kind of statement comes without parsing it.
    pub fn next_words<'a>(&self, source: &'a str) -> (Option<&'a str>, Option<&'a str>) {
        let rest = &self.tokens[self.position.min(self.stream_len())..self.stream_len()];
        let mut words = rest
            .iter()
            .skip_while(|token| {
                matches!(token.contents, TokenContents::Eol | TokenContents::Comment | TokenContents::Semicolon)
            })
            .map_while(|token| token.is_item().then(|| token.text(source)));
        (words.next(), words.next())
    }

    /// The number of tokens the statements are parsed from (all but the closing `Eof`).
    fn stream_len(&self) -> usize {
        match self.tokens.last() {
            Some(last) if last.contents == TokenContents::Eof => self.tokens.len() - 1,
            _ => self.tokens.len(),
        }
    }
}

/// nu's lite parser: a block whose last token, skipping trailing comment
/// lines, is a `|` has a pipeline with no end (`ls |`, `ls |\n# c`,
/// `alias x = ls |`), whatever absorbed the pipe.
fn check_dangling_pipe(working_set: &WorkingSet<'_>, tokens: &[Token]) {
    if last_non_comment_token(tokens) == Some(TokenContents::Pipe)
        && let Some(last) = tokens.iter().rev().find(|token| token.contents == TokenContents::Pipe)
    {
        working_set.error(
            Diagnostic::new(ErrorKind::UnexpectedEof("command after `|`"), last.span)
                .with_context("pipeline")
                .with_help("the pipeline has no end: add a command after the `|` or remove it"),
        );
    }
}

/// Where [`parse_statements`] puts the pipelines it parses: a `Vec` for [`parse_block`],
/// which leaves every diagnostic in the working set for the whole parse to collect, or a
/// [`StreamingSink`] for [`BlockStatements::parse`], which hands each statement over with
/// its own diagnostics.
trait StatementSink<'a> {
    /// Take the next pipeline. `errors_before` is [`WorkingSet::error_count`] from just
    /// before it was parsed: every diagnostic recorded since, those of its nested blocks
    /// included, is the statement's. Returns whether to go on.
    fn push(&mut self, pipeline: Pipeline<'a>, errors_before: usize) -> bool;
    /// Take the placeholder for a statement covering `span` that failed to
    /// parse. Returns whether to go on.
    fn push_garbage(&mut self, span: Span, leading_comments: Vec<Comment>, errors_before: usize) -> bool;
    /// The pipeline pushed last, while it can still be amended (its
    /// terminator, a comment after it on its line); `None` once it was handed over.
    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>>;
}

/// The whole block at once ([`parse_block`]): the diagnostics stay in the working set.
impl<'a> StatementSink<'a> for Vec<Pipeline<'a>> {
    fn push(&mut self, pipeline: Pipeline<'a>, _errors_before: usize) -> bool {
        Vec::push(self, pipeline);
        true
    }

    fn push_garbage(&mut self, span: Span, _leading_comments: Vec<Comment>, _errors_before: usize) -> bool {
        Vec::push(self, garbage_pipeline(span));
        true
    }

    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>> {
        self.as_mut_slice().last_mut()
    }
}

/// Hands each pipeline to a [`BlockStatements::parse`] sink with the diagnostics parsing it
/// reported, taken out of the working set.
struct StreamingSink<'s, 'w, 'a> {
    /// The caller's sink: takes a statement and says whether to go on.
    sink: &'s mut dyn FnMut(Pipeline<'a>, Vec<Diagnostic>) -> bool,
    /// The working set the statements are parsed in, where their diagnostics are recorded.
    working_set: &'w WorkingSet<'a>,
}

impl<'a> StatementSink<'a> for StreamingSink<'_, '_, 'a> {
    fn push(&mut self, pipeline: Pipeline<'a>, errors_before: usize) -> bool {
        let diagnostics = self.working_set.take_errors_from(errors_before);
        (self.sink)(pipeline, diagnostics)
    }

    fn push_garbage(&mut self, span: Span, leading_comments: Vec<Comment>, errors_before: usize) -> bool {
        // The statement's doc comments go with it, for whoever parses it again.
        let mut garbage = garbage_pipeline(span);
        garbage.leading_comments = leading_comments;
        self.push(garbage, errors_before)
    }

    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>> {
        // Each pipeline went to the sink when it was pushed.
        None
    }
}

/// Where [`parse_statements`] stands between two statements, so that a block's statements can
/// be parsed a few at a time ([`BlockStatements`]).
#[derive(Debug)]
struct StatementsState {
    /// Comments on lines of their own since the last statement: the next statement's leading
    /// comments, unless a blank line comes first.
    pending: Vec<Comment>,
    /// A `|` that no command followed before a blank line, which nu hands to the next command.
    carried_pipe: Option<Span>,
    /// What the last token was (`Item` for a whole statement): an `Eol` after an `Eol` is a
    /// blank line, and a `;` or a comment right after a statement belongs to it.
    last: TokenContents,
    /// Whether a statement was pushed. A comment that is not first on its line trails the
    /// statement pushed last, if there is one; otherwise it waits for the next statement.
    pushed: bool,
}

/// The start of a block counts as the start of a line.
impl Default for StatementsState {
    fn default() -> Self {
        Self { pending: Vec::new(), carried_pipe: None, last: TokenContents::Eol, pushed: false }
    }
}

/// The statements of a block, pushed to `out` as they are parsed, until `out`
/// says to stop. Comments on lines of their own before a statement become its
/// leading comments.
///
/// A loop over the tokens rather than a combinator: what an `Eol`, `;` or
/// comment means depends on the token before it (`state.last`), and the state
/// must survive a stop so that a later call goes on from it.
fn parse_statements<'a>(tokens: &mut Tokens<'_, 'a>, state: &mut StatementsState, out: &mut dyn StatementSink<'a>) {
    let working_set = tokens.working_set;
    let StatementsState { pending, carried_pipe, last, pushed } = state;
    while let Some(token) = tokens.peek_token() {
        match token.contents {
            TokenContents::Eol => {
                if *last == TokenContents::Eol {
                    pending.clear();
                }
                tokens.next_token();
                *last = TokenContents::Eol;
            }
            TokenContents::Semicolon => {
                // A parsed pipeline took the `;` right after it already, so this sets the
                // terminator of a garbage statement.
                if !matches!(last, TokenContents::Eol | TokenContents::Semicolon)
                    && let Some(pipeline) = out.last_mut()
                    && pipeline.terminator.is_none()
                {
                    pipeline.terminator = Some(token.span);
                }
                tokens.next_token();
                *last = TokenContents::Semicolon;
            }
            TokenContents::Comment => {
                working_set.add_comment(token.span);
                if *pushed && *last != TokenContents::Eol {
                    if let Some(pipeline) = out.last_mut() {
                        pipeline.trailing_comments.push(Comment { span: token.span });
                    }
                } else {
                    pending.push(Comment { span: token.span });
                }
                tokens.next_token();
                *last = TokenContents::Comment;
            }
            _ => {
                let start = tokens.position();
                let start_span = token.span;
                let errors_before = working_set.error_count();
                // The state is up to date before each push: `out` may stop the loop there, and a
                // later call goes on from the state.
                let go_on = match parse_pipeline(tokens, pending, carried_pipe) {
                    Ok(Some(mut pipeline)) => {
                        *pushed = true;
                        *last = TokenContents::Item;
                        // A comment right after the statement, on its line, is the statement's
                        // (for a definition, part of its description), so it goes with it.
                        if let Some(comment) =
                            tokens.peek_token().filter(|next| next.contents == TokenContents::Comment)
                        {
                            working_set.add_comment(comment.span);
                            pipeline.trailing_comments.push(Comment { span: comment.span });
                            tokens.next_token();
                            *last = TokenContents::Comment;
                        }
                        out.push(pipeline, errors_before)
                    }
                    Ok(None) => {
                        pending.clear();
                        *last = TokenContents::Item;
                        true
                    }
                    Err(error) => {
                        working_set.error(into_diagnostic(error));
                        tokens.reset_to(start);
                        let end = skip_to_statement_end(tokens);
                        *pushed = true;
                        *last = TokenContents::Item;
                        out.push_garbage(start_span.merge(end), std::mem::take(pending), errors_before)
                    }
                };
                if !go_on {
                    return;
                }
            }
        }
    }
}

/// One pipeline (nu's `parse_pipeline`): commands separated by `|`. `None`
/// when there was no command at all (a lone `|` before a blank line), the `|`
/// then left in `carried_pipe`: nu's lite parser keeps it for the next command,
/// which starts with it as with a leading `|`. A `;` right after a `|` never
/// gets here: the lexer refuses it, as nu's does. The pipeline takes
/// `leading_comments` only when it parses.
fn parse_pipeline<'a>(
    tokens: &mut Tokens<'_, 'a>,
    leading_comments: &mut Vec<Comment>,
    carried_pipe: &mut Option<Span>,
) -> ParseResult<Option<Pipeline<'a>>> {
    let working_set = tokens.working_set;
    // The lite parse first: collect the commands, then parse them, because a
    // command is parsed differently when it is one element of a longer pipeline.
    let mut lite_commands: Vec<(Option<Span>, LiteCommand)> = Vec::new();
    let mut trailing_comments = Vec::new();
    let mut pipe: Option<Span> = carried_pipe.take();
    // Each turn takes the `|`s before a command, then the command. A `|` on the command's
    // line is left for the next turn; an `e>|` is taken with the command (`pipe_after`).
    'commands: loop {
        // A pipeline may start with `|` (`( | str join)`) and `a | | b` is
        // `a | b`: the empty commands are dropped.
        while let Some(token) = tokens.peek_token().filter(|token| token.contents == TokenContents::Pipe) {
            pipe = Some(token.span);
            tokens.next_token();
            if let AfterPipe::Dangling = after_pipe(tokens, true, &mut trailing_comments)? {
                break 'commands;
            }
        }
        if pipe.is_none() && !lite_commands.is_empty() {
            // After a command: the pipeline goes on only through a `|` on a later line.
            if !take_pipe_on_later_line(tokens, &mut trailing_comments)? {
                break;
            }
            continue;
        }
        // nu's lite parser reads attribute lines where an `@` item follows an end of line or a
        // `;`: before a statement's first command, and before a command starting a line.
        let starts_line = tokens
            .position()
            .checked_sub(1)
            .and_then(|previous| tokens.all().get(previous))
            .is_some_and(|previous| previous.contents == TokenContents::Eol);
        let lite_command = parse_lite_command(tokens, pipe.is_none() || starts_line)?;
        trailing_comments.extend(lite_command.comments.iter().copied());
        let pipe_after = lite_command.pipe_after;
        lite_commands.push((pipe.take(), lite_command));
        if pipe_after.is_some() {
            pipe = pipe_after;
            // After an `e>|` nu goes on to a later line only through a `|` that starts it.
            if let AfterPipe::Dangling = after_pipe(tokens, false, &mut trailing_comments)? {
                break;
            }
        }
    }
    if lite_commands.is_empty() {
        // Only pipes (`|` and a blank line): nu drops the empty command.
        *carried_pipe = pipe;
        return Ok(None);
    }
    let single = lite_commands.len() == 1;
    let mut elements: Vec<PipelineElement<'a>> = Vec::with_capacity(lite_commands.len());
    for (pipe, lite_command) in &lite_commands {
        let (expr, redirection) = parse_pipeline_element(working_set, lite_command, !single)?;
        let start = pipe.map_or(expr.span.start, |pipe| pipe.start);
        let end = redirection.as_ref().map_or(expr.span.end, |redirection| redirection.span().end.max(expr.span.end));
        elements.push(PipelineElement { span: Span::new(start, end), pipe: *pipe, expr, redirection });
    }
    let span = elements[0].span.merge(elements.last().map_or(elements[0].span, |element| element.span));
    let terminator = match tokens.peek_token() {
        Some(token) if token.contents == TokenContents::Semicolon => {
            tokens.next_token();
            Some(token.span)
        }
        _ => None,
    };
    let leading_comments = std::mem::take(leading_comments);
    Ok(Some(Pipeline { span, elements, leading_comments, trailing_comments, terminator }))
}

/// One command of a pipeline, as collected by the lite parse (nu's
/// `parse_pipeline_element`). `in_pipeline` is set when it is one of several
/// elements; a lone command may be a statement (nu's `parse_builtin_commands`).
fn parse_pipeline_element<'a>(
    working_set: &WorkingSet<'a>,
    lite_command: &LiteCommand,
    in_pipeline: bool,
) -> ParseResult<(Expression<'a>, Option<PipelineRedirection<'a>>)> {
    let position = if in_pipeline { Position::Element } else { Position::Statement };
    let expr = match lite_command.attributes.as_slice() {
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
                .flatten()
                .map(|token| match assignment_operator(working_set.get_span_contents(token.span)) {
                    Some(operator) => Token { contents: TokenContents::AssignmentOperator(operator), span: token.span },
                    None => *token,
                })
                .chain(lite_command.parts.iter().copied())
                .collect();
            parse_expression(Tokens::new(working_set, &items, lite_command.end), position)?
        }
        attribute_lines => {
            let attributes = attribute_lines
                .iter()
                .map(|attribute_line| parse_attribute(working_set, attribute_line))
                .collect::<ParseResult<Vec<_>>>()?;
            let words: Vec<&str> =
                lite_command.parts.iter().take(2).map(|token| working_set.get_span_contents(token.span)).collect();
            let is_definition = matches!(words.as_slice(), ["def" | "extern", ..] | ["export", "def" | "extern"]);
            let item = match (is_definition, lite_command.parts.first()) {
                (true, Some(first)) if in_pipeline => {
                    return Err(cut(Diagnostic::new(
                        ErrorKind::KeywordInPipeline(working_set.get_span_contents(first.span).to_string()),
                        first.span,
                    )));
                }
                (true, _) => parse_builtin_commands(lite_command.tokens(working_set))?,
                (false, Some(first)) => {
                    return Err(cut(Diagnostic::message("attributes must be followed by a definition", first.span)
                        .with_help("only `def`, `extern`, `export def` and `export extern` take attributes")));
                }
                (false, None) => {
                    let last = attributes.last().map_or(Span::point(lite_command.end), |attribute| attribute.span);
                    return Err(cut(Diagnostic::message("attributes must be followed by a definition", last.past())
                        .with_help("put a `def` or `extern` on the line after the attributes")));
                }
            };
            let span = attributes[0].span.merge(item.span);
            Expression::new(Expr::AttributeBlock(AttributeBlock { attributes, item: Box::new(item) }), span)
        }
    };
    // nu's command covers all of its items, those it ignores included (`try {} --`).
    let mut expr = expr;
    if let Some(last) = lite_command.parts.last() {
        expr.span.end = expr.span.end.max(last.span.end);
    }
    let redirection = parse_redirection(working_set, lite_command)?;
    if let Expr::ExportEnv(_) = expr.expr
        && redirection.is_some()
    {
        // nu never looks at a redirection on `export-env`: it is ignored text, not
        // refused, so this comes before `rejects_redirection`.
        for (operator, target) in &lite_command.redirections {
            working_set.add_ignored(operator.span);
            if let Some(target) = target {
                working_set.add_ignored(target.span);
            }
        }
        return Ok((expr, None));
    }
    if redirection.is_some() && rejects_redirection(&expr) {
        let at = lite_command.redirections.first().map_or(expr.span, |(operator, _)| operator.span);
        return Err(cut(Diagnostic::message("this statement cannot be redirected", at)));
    }
    Ok((expr, redirection))
}

/// The statements nu refuses to redirect (nu's `redirecting_builtin_error`).
fn rejects_redirection(expr: &Expression<'_>) -> bool {
    match &expr.expr {
        Expr::Def(_)
        | Expr::Extern(_)
        | Expr::Let(_)
        | Expr::Mut(_)
        | Expr::Const(_)
        | Expr::For(_)
        | Expr::Alias(_)
        | Expr::Module(_)
        | Expr::Use(_)
        | Expr::Export(_)
        | Expr::AttributeBlock(_) => true,
        // `overlay <anything>` is refused by name before its arguments are looked at.
        Expr::Call(call) => {
            call.head.name.split(' ').next() == Some("overlay")
                || keyword_signature_of_call(call).is_some_and(|signature| !signature.redirectable)
        }
        _ => false,
    }
}

/// The redirections of a command (nu's `parse_redirection`): one stream
/// (`o> f`, `o+e> f`, `e>| cmd`), or stdout and stderr separately (`o> f e> g`,
/// in either order). Redirecting the same stream twice is an error.
fn parse_redirection<'a>(
    working_set: &WorkingSet<'a>,
    lite_command: &LiteCommand,
) -> ParseResult<Option<PipelineRedirection<'a>>> {
    let mut redirection: Option<PipelineRedirection<'a>> = None;
    for (operator, target) in &lite_command.redirections {
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
                Some(PipelineRedirection::Single { source: RedirectionSource::Stdout, target: stdout }),
                RedirectionSource::Stderr,
            ) => PipelineRedirection::Separate { out: stdout, err: target },
            (
                Some(PipelineRedirection::Single { source: RedirectionSource::Stderr, target: stderr }),
                RedirectionSource::Stdout,
            ) => PipelineRedirection::Separate { out: target, err: stderr },
            (Some(previous), _) => {
                return Err(cut(Diagnostic::message("multiple redirections of the same stream", operator.span)
                    .with_help(format!("the stream is already redirected at {}", previous.span()))));
            }
        });
    }
    Ok(redirection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DeclKind, ParseConfig};

    /// The pipelines of `source`, which must parse.
    fn parse(source: &str) -> Vec<Pipeline<'_>> {
        let (ast, diagnostics) = crate::parser::parse(source, &ParseConfig::new());
        assert!(diagnostics.is_empty(), "{source:?}: {diagnostics:?}");
        ast.block.pipelines
    }

    /// The number of elements of each pipeline of `source`.
    fn pipelines(source: &str) -> Vec<usize> {
        parse(source).iter().map(|pipeline| pipeline.elements.len()).collect()
    }

    #[test]
    fn line_leading_pipe_after_one_blank_line_continues() {
        assert_eq!(pipelines("[1 2 3] |\n\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] |\n# c\n\n# d\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] | # c\n\n| length"), [2]);
        assert_eq!(pipelines("[1 2 3] |\n\n\n| length"), [1, 1]);
        assert_eq!(pipelines("[1 2 3]\n\n| length"), [1, 1]);
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
        let [Pipeline { elements, .. }] = &pipelines[..] else { panic!("{source:?}: {pipelines:?}") };
        let [PipelineElement { expr: Expression { expr: Expr::AttributeBlock(block), .. }, .. }] = &elements[..] else {
            panic!("{source:?}: {elements:?}")
        };
        block.attributes.iter().map(|attribute| attribute.arguments.len()).collect()
    }

    #[test]
    fn attribute_lines_where_a_line_starts() {
        // A command that starts a line may have attribute lines, after a `|` too.
        assert_eq!(attribute_arguments("|\n@search-terms foo\ndef bar [] {}"), [1]);
        assert_eq!(pipelines("ls | length |\n@search-terms foo\ndef bar [] {}"), [3]);
        // A `|` that starts the next line goes on with the attribute line.
        assert_eq!(attribute_arguments("@search-terms foo\n| bar\ndef baz [] {}"), [3]);
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

        fn is_builtin_decl(&self, _: &str) -> bool {
            false
        }
    }

    /// The text of each statement of the block covering `span` of `source`.
    fn statements(source: &str, span: Span, subexpression: bool) -> Vec<&str> {
        let (mut statements, _) = BlockStatements::new(source, span, NoCommands, subexpression).unwrap();
        let mut texts = Vec::new();
        statements.parse(source, NoCommands, &mut |pipeline, diagnostics| {
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
        assert_eq!(statements(source, inner, true), ["echo a\n  b # c\n  | length"]);
        assert_eq!(statements(source, inner, false), ["echo a", "b # c\n  | length"]);
    }
}
