//! Blocks, pipelines and pipeline elements (nu-parser's `parse_pipelines.rs`).

use crate::ast::{
    AttributeBlock, Block, Comment, Expr, Expression, Pipeline, PipelineElement, PipelineRedirection, RedirectionTarget,
};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut, into_diagnostic};
use crate::lex::{RedirectionSource, Token, TokenContents, assignment_operator};
use crate::span::Span;

use super::WorkingSet;
use super::lite_parser::{
    AfterPipe, LiteCommand, after_pipe, last_non_comment_token, parse_lite_command, skip_to_statement_end,
    take_pipe_on_later_line,
};
use super::parse_calls::{keyword_signature_of_call, parse_attribute};
use super::parse_def::{PredeclaredDef, collect_predecls, parse_def_predecl};
use super::parse_expressions::{ExpectedShape, Position, parse_builtin_commands, parse_expression, parse_value};
use super::parse_helpers::garbage_pipeline;
use super::tokens::Tokens;

/// The pipelines of a block covering `span` (nu's `parse_block`).
///
/// Errors are recorded in the working set and the offending statement becomes
/// an [`Expr::Garbage`] pipeline, so parsing continues with the next line.
pub fn parse_block<'a>(mut tokens: Tokens<'_, 'a>, span: Span) -> Block<'a> {
    let working_set = tokens.working_set;
    parse_def_predecl(working_set, tokens.all());
    check_dangling_pipe(working_set, tokens.all());
    let mut pipelines: Vec<Pipeline<'a>> = Vec::new();
    parse_statements(&mut tokens, &mut pipelines);
    Block { span, pipelines }
}

/// What [`parse_block_streaming`] hands over, in source order: first every
/// definition of the block, then its statements one at a time.
pub trait BlockSink<'a> {
    /// A `def` or `extern` of the block, found before any statement is parsed
    /// (nu's `parse_def_predecl`), with its signature parsed.
    fn predecl(&mut self, def: PredeclaredDef<'a>);

    /// The next statement of the block, parsed only after every earlier one
    /// was handed over, so that what an earlier statement declares (a `use`,
    /// an `alias`) is known to the [`CommandLookup`](super::working_set::CommandLookup)
    /// while this one is parsed. `diagnostics` holds what parsing it reported;
    /// when it is not empty, `pipeline` may be a garbage placeholder that spans
    /// the statement. Returns whether to go on with the next statement.
    fn statement(&mut self, pipeline: Pipeline<'a>, diagnostics: Vec<Diagnostic>) -> bool;
}

/// The pipelines of a block covering `span`, handed to `sink` one statement
/// at a time (see [`BlockSink`]).
///
/// Diagnostics that concern the block as a whole (a definition declared twice,
/// a `|` that ends the block) are returned before `sink` sees anything, so the
/// caller can parse the block another way. Neither a statement's terminator
/// nor the comments after it on its line are handed over.
pub fn parse_block_streaming<'a>(
    mut tokens: Tokens<'_, 'a>,
    sink: &mut dyn BlockSink<'a>,
) -> Result<(), Vec<Diagnostic>> {
    let working_set = tokens.working_set;
    let before = working_set.error_count();
    let predecls = collect_predecls(working_set, tokens.all());
    check_dangling_pipe(working_set, tokens.all());
    let block_errors = working_set.take_errors_from(before);
    if !block_errors.is_empty() {
        return Err(block_errors);
    }
    for def in predecls {
        sink.predecl(def);
    }
    parse_statements(&mut tokens, &mut StreamingSink { sink, working_set, handed_over: false });
    Ok(())
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

/// Where [`parse_statements`] puts the pipelines it parses.
trait StatementSink<'a> {
    /// Take the next pipeline. `errors_before` is the number of diagnostics
    /// recorded before it was parsed. Returns whether to go on.
    fn push(&mut self, pipeline: Pipeline<'a>, errors_before: usize) -> bool;
    /// Take the placeholder for a statement covering `span` that failed to
    /// parse. Returns whether to go on.
    fn push_garbage(&mut self, span: Span, leading_comments: Vec<Comment>, errors_before: usize) -> bool;
    /// The pipeline pushed last, while it can still be amended (its
    /// terminator, a comment after it on its line).
    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>>;
    /// Whether a pipeline was pushed.
    fn has_last(&self) -> bool;
}

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

    fn has_last(&self) -> bool {
        !self.is_empty()
    }
}

/// Hands each pipeline to a [`BlockSink`] with the diagnostics parsing it
/// reported.
struct StreamingSink<'s, 'w, 'a> {
    sink: &'s mut dyn BlockSink<'a>,
    working_set: &'w WorkingSet<'a>,
    handed_over: bool,
}

impl<'a> StatementSink<'a> for StreamingSink<'_, '_, 'a> {
    fn push(&mut self, pipeline: Pipeline<'a>, errors_before: usize) -> bool {
        let diagnostics = self.working_set.take_errors_from(errors_before);
        self.handed_over = true;
        self.sink.statement(pipeline, diagnostics)
    }

    fn push_garbage(&mut self, span: Span, leading_comments: Vec<Comment>, errors_before: usize) -> bool {
        // The statement's doc comments go with it, for whoever parses it again.
        let mut garbage = garbage_pipeline(span);
        garbage.leading_comments = leading_comments;
        self.push(garbage, errors_before)
    }

    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>> {
        None
    }

    fn has_last(&self) -> bool {
        self.handed_over
    }
}

/// The statements of a block, pushed to `out` as they are parsed, until `out`
/// says to stop. Comments on lines of their own before a statement become its
/// leading comments.
fn parse_statements<'a>(tokens: &mut Tokens<'_, 'a>, out: &mut dyn StatementSink<'a>) {
    let working_set = tokens.working_set;
    let mut pending: Vec<Comment> = Vec::new();
    let mut last = TokenContents::Eol;
    while let Some(token) = tokens.peek_token() {
        match token.contents {
            TokenContents::Eol => {
                if last == TokenContents::Eol {
                    pending.clear();
                }
                tokens.next_token();
                last = TokenContents::Eol;
            }
            TokenContents::Semicolon => {
                if !matches!(last, TokenContents::Eol | TokenContents::Semicolon)
                    && let Some(pipeline) = out.last_mut()
                    && pipeline.terminator.is_none()
                {
                    pipeline.terminator = Some(token.span);
                }
                tokens.next_token();
                last = TokenContents::Semicolon;
            }
            TokenContents::Comment => {
                working_set.add_comment(token.span);
                if out.has_last() && last != TokenContents::Eol {
                    if let Some(pipeline) = out.last_mut() {
                        pipeline.trailing_comments.push(Comment { span: token.span });
                    }
                } else {
                    pending.push(Comment { span: token.span });
                }
                tokens.next_token();
                last = TokenContents::Comment;
            }
            _ => {
                let start = tokens.position();
                let start_span = token.span;
                let errors_before = working_set.error_count();
                match parse_pipeline(tokens, &mut pending) {
                    Ok(Some(mut pipeline)) => {
                        // A comment right after the statement, on its line, is the statement's
                        // (for a definition, part of its description), so it goes with it.
                        if let Some(comment) =
                            tokens.peek_token().filter(|next| next.contents == TokenContents::Comment)
                        {
                            working_set.add_comment(comment.span);
                            pipeline.trailing_comments.push(Comment { span: comment.span });
                            tokens.next_token();
                            if !out.push(pipeline, errors_before) {
                                return;
                            }
                            last = TokenContents::Comment;
                            continue;
                        }
                        if !out.push(pipeline, errors_before) {
                            return;
                        }
                    }
                    Ok(None) => pending.clear(),
                    Err(error) => {
                        working_set.error(into_diagnostic(error));
                        tokens.reset_to(start);
                        let end = skip_to_statement_end(tokens);
                        if !out.push_garbage(start_span.merge(end), std::mem::take(&mut pending), errors_before) {
                            return;
                        }
                    }
                }
                last = TokenContents::Item;
            }
        }
    }
}

/// One pipeline (nu's `parse_pipeline`): commands separated by `|`. `None`
/// when there was no command at all (a lone `|` before a blank line). A `;`
/// right after a `|` never gets here: the lexer refuses it, as nu's does.
/// The pipeline takes `leading_comments` only when it parses.
fn parse_pipeline<'a>(
    tokens: &mut Tokens<'_, 'a>,
    leading_comments: &mut Vec<Comment>,
) -> ParseResult<Option<Pipeline<'a>>> {
    let working_set = tokens.working_set;
    // The lite parse first: collect the commands, then parse them, because a
    // command is parsed differently when it is one element of a longer pipeline.
    let mut lite_commands: Vec<(Option<Span>, LiteCommand)> = Vec::new();
    let mut trailing_comments = Vec::new();
    let mut pipe: Option<Span> = None;
    'commands: loop {
        // A pipeline may start with `|` (`( | str join)`) and `a | | b` is
        // `a | b`: the empty commands are dropped.
        while let Some(token) = tokens.peek_token().filter(|token| token.contents == TokenContents::Pipe) {
            pipe = Some(token.span);
            tokens.next_token();
            if let AfterPipe::Dangling = after_pipe(tokens, &mut trailing_comments)? {
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
        let lite_command = parse_lite_command(tokens, pipe.is_none())?;
        trailing_comments.extend(lite_command.comments.iter().copied());
        let pipe_after = lite_command.pipe_after;
        lite_commands.push((pipe.take(), lite_command));
        if pipe_after.is_some() {
            pipe = pipe_after;
            if let AfterPipe::Dangling = after_pipe(tokens, &mut trailing_comments)? {
                break;
            }
        }
    }
    if lite_commands.is_empty() {
        // Only pipes (`|` and a blank line): nu drops the empty command.
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
    let redirection = parse_redirection(working_set, lite_command)?;
    if let Expr::ExportEnv(_) = expr.expr
        && redirection.is_some()
    {
        // nu never looks at a redirection on `export-env`.
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
        | Expr::ExportEnv(_)
        | Expr::AttributeBlock(_) => true,
        // `overlay <anything>` is refused by name before its arguments are looked at.
        Expr::Call(call) => {
            call.head.name.split(' ').next() == Some("overlay")
                || keyword_signature_of_call(call).is_some_and(|signature| !signature.redirectable)
        }
        _ => false,
    }
}

/// The redirections of a command (nu's `parse_redirection`): one stream, or
/// stdout and stderr separately.
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
