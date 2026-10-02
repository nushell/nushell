//! The lite parse: grouping a block's tokens into commands (nu-parser's
//! `lite_parser.rs`).
//!
//! This decides where commands start and end, attaches comments, absorbs the
//! rest of a line after an assignment operator, collects redirections and
//! attribute lines. It never looks inside an item.
//!
//! The rules for newlines around `|` are nu's exactly: a `|` continues the
//! previous line when only an end of line, or comment lines each on their own
//! line, stand between them (`a\n# c\n| b`); after a `|` the pipeline goes on
//! across one end of line and comment lines (`a |\n# c\n b`). A blank line on
//! either side closes the pipeline: `a\n\n| b` and `a |\n\n b` are two
//! pipelines each (the trailing `|` of the first is dropped silently, as nu
//! does), and a `|` that nothing but comments follow at the end of a block is
//! an error.

use winnow::Parser;
use winnow::combinator::{opt, peek, preceded, repeat, terminated};

use crate::ast::Comment;
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseResult, cut};
use crate::lex::{RedirectionOperator, RedirectionSource, Token, TokenContents};
use crate::span::{Span, Spanned};

use super::WorkingSet;
use super::tokens::{Tokens, comment, eol, pipe};

/// One command as grouped by the lite parse, before its items are interpreted.
#[derive(Debug, Default)]
pub struct LiteCommand {
    /// The items and, after an assignment operator, everything to the end of the line.
    pub parts: Vec<Token>,
    /// Byte offset just past the last part.
    pub end: usize,
    /// Preceding `@attribute` lines.
    pub attributes: Vec<Vec<Token>>,
    /// Redirections and their file targets (`None` for `e>|`).
    pub redirections: Vec<(Spanned<RedirectionOperator>, Option<Token>)>,
    /// The span of an `e>|` (or `o+e>|`) that ended the command, if any.
    pub pipe_after: Option<Span>,
    /// Comments found between the command's tokens.
    pub comments: Vec<Comment>,
}

impl LiteCommand {
    /// A stream over the parts.
    pub fn tokens<'t, 'a>(&'t self, working_set: &'t WorkingSet<'a>) -> Tokens<'t, 'a> {
        Tokens::new(working_set, &self.parts, self.end)
    }
}

/// Skip tokens up to (not including) the next `Eol`/`;`, returning the span
/// of the last token skipped.
pub fn skip_to_statement_end(tokens: &mut Tokens<'_, '_>) -> Span {
    let mut end = None;
    while let Some(token) = tokens.peek_token() {
        if matches!(token.contents, TokenContents::Eol | TokenContents::Semicolon) {
            break;
        }
        end = Some(token.span);
        tokens.next_token();
    }
    end.unwrap_or_else(|| Span::point(tokens.here().start))
}

/// The last token that is not part of a trailing `([Comment]+ [Eol])*`
/// sequence: nu's `last_non_comment_token`, used to tell `ls |\n` (fine)
/// from `ls |` and `ls |\n# c` (a pipeline with no end).
pub fn last_non_comment_token(tokens: &[Token]) -> Option<TokenContents> {
    let mut expect = TokenContents::Comment;
    for token in tokens.iter().rev() {
        match (token.contents, expect) {
            (TokenContents::Comment, TokenContents::Comment | TokenContents::Eol) => expect = TokenContents::Eol,
            (TokenContents::Eol, TokenContents::Eol) => expect = TokenContents::Comment,
            (kind, _) => return Some(kind),
        }
    }
    None
}

/// The comment lines before a `|` on a later line that continues the
/// pipeline: exactly one end of line, then any number of comment lines, then
/// the pipe (`Eol (Comment Eol)* Pipe`), which is left for the caller. A blank
/// line in between closes the pipeline instead.
fn pipe_on_later_line(tokens: &mut Tokens<'_, '_>) -> ParseResult<Vec<Token>> {
    preceded(eol, terminated(repeat(0.., terminated(comment, eol)), peek(pipe))).parse_next(tokens)
}

/// Consume the lines up to a `|` that continues the pipeline on a later
/// line, recording their comments. `false` when no such `|` follows.
pub fn take_pipe_on_later_line(tokens: &mut Tokens<'_, '_>, comments: &mut Vec<Comment>) -> ParseResult<bool> {
    let Some(comment_lines) = opt(pipe_on_later_line).parse_next(tokens)? else { return Ok(false) };
    record_comments(tokens.working_set, &comment_lines, comments);
    Ok(true)
}

/// Record comment tokens both in the working set and in `comments`.
fn record_comments(working_set: &WorkingSet<'_>, comment_tokens: &[Token], comments: &mut Vec<Comment>) {
    for token in comment_tokens {
        working_set.add_comment(token.span);
        comments.push(Comment { span: token.span });
    }
}

/// What follows a `|` (just consumed) after its continuation lines.
pub enum AfterPipe {
    /// A command follows.
    Command,
    /// A blank line, a `;` (after a redirection pipe such as `e>|`; the lexer
    /// refuses one after a plain `|`) or the end of the block: the pipeline ends
    /// here and the `|` is dropped.
    Dangling,
}

/// After a `|`: the comments on its line, then one end of line and any
/// comment lines (`Comment* [Eol (Comment Eol)*]`), the way nu's lite parser
/// keeps a pipeline open across them.
pub fn after_pipe(tokens: &mut Tokens<'_, '_>, comments: &mut Vec<Comment>) -> ParseResult<AfterPipe> {
    let same_line: Vec<Token> = repeat(0.., comment).parse_next(tokens)?;
    let later_lines: Option<Vec<Token>> =
        opt(preceded(eol, repeat(0.., terminated(comment, eol)))).parse_next(tokens)?;
    record_comments(tokens.working_set, &same_line, comments);
    record_comments(tokens.working_set, &later_lines.unwrap_or_default(), comments);
    // A `|` that only comments follow at the end of the block is reported
    // once, by `parse_block`, whichever command absorbed it.
    match tokens.peek_token().map(|token| token.contents) {
        None | Some(TokenContents::Eol | TokenContents::Semicolon) => Ok(AfterPipe::Dangling),
        Some(_) => Ok(AfterPipe::Command),
    }
}

/// Collect the tokens of one command. `first` is set for the first command of
/// a pipeline, the only place attribute lines can precede it.
pub fn parse_lite_command(tokens: &mut Tokens<'_, '_>, first: bool) -> ParseResult<LiteCommand> {
    let working_set = tokens.working_set;
    let mut lite_command = LiteCommand::default();
    if first {
        lite_attribute_lines(tokens, &mut lite_command)?;
    }
    // After `=` everything to the end of the line belongs to the command.
    let mut absorbing = false;
    while let Some(&token) = tokens.peek_token() {
        match token.contents {
            TokenContents::Item => lite_command.parts.push(token),
            TokenContents::AssignmentOperator(_) => {
                lite_command.parts.push(token);
                absorbing = true;
            }
            TokenContents::Pipe | TokenContents::PipePipe | TokenContents::Redirection(_) if absorbing => {
                lite_command.parts.push(token)
            }
            TokenContents::Eol if absorbing => {
                // `$x = a |\n b` and `$x = a\n | b` continue the assignment's pipeline.
                // Like nu (`last_non_comment_token`), comment lines may come after the
                // `|`, but a blank line ends the command: `let x = 1 |\n\nls` leaves
                // `1 |` without an end.
                let ends_with_pipe =
                    last_non_comment_token(&tokens.all()[..tokens.position()]) == Some(TokenContents::Pipe);
                if ends_with_pipe {
                    tokens.next_token();
                    continue;
                }
                if take_pipe_on_later_line(tokens, &mut lite_command.comments)? {
                    continue;
                }
                break;
            }
            TokenContents::Comment => {
                working_set.add_comment(token.span);
                lite_command.comments.push(Comment { span: token.span });
            }
            TokenContents::Redirection(operator) => {
                tokens.next_token();
                if lite_command.parts.is_empty() {
                    return Err(cut(Diagnostic::message("unexpected redirection: nothing to redirect", token.span)));
                }
                if operator.is_pipe() {
                    lite_command.redirections.push((Spanned::new(operator, token.span), None));
                    lite_command.pipe_after = Some(token.span);
                    break;
                }
                let target = tokens.expect_item("redirection target")?;
                lite_command.redirections.push((Spanned::new(operator, token.span), Some(target)));
                continue;
            }
            TokenContents::Pipe => break,
            TokenContents::PipePipe => {
                return Err(cut(Diagnostic::new(
                    ErrorKind::ShellSyntax { found: "||", use_instead: "or" },
                    token.span,
                )
                .with_help("use `or` for boolean logic, or `try { } catch { }` to run a fallback command")));
            }
            TokenContents::Eol | TokenContents::Semicolon | TokenContents::Eof => break,
        }
        tokens.next_token();
    }
    if lite_command.parts.is_empty() && lite_command.attributes.is_empty() && lite_command.redirections.is_empty() {
        return Err(cut(Diagnostic::expected("command", tokens.here())));
    }
    lite_command.end = lite_command.parts.last().map_or(tokens.here().start, |token| token.span.end);
    Ok(lite_command)
}

/// Leading `@name arguments` lines, each up to the end of its line or a `;`. Like
/// nu, everything on the line is an argument of the attribute, pipes and
/// redirections included, and the definition must follow on the very next
/// line: a blank or comment line in between is an error.
fn lite_attribute_lines(tokens: &mut Tokens<'_, '_>, lite_command: &mut LiteCommand) -> ParseResult<()> {
    let working_set = tokens.working_set;
    while tokens.peek_token().is_some_and(|token| {
        token.contents == TokenContents::Item && working_set.get_span_contents(token.span).starts_with('@')
    }) {
        let mut attribute_line = Vec::new();
        let mut ends_with_pipe = false;
        while let Some(&token) = tokens.peek_token() {
            match token.contents {
                TokenContents::Comment => {
                    working_set.add_comment(token.span);
                    lite_command.comments.push(Comment { span: token.span });
                }
                TokenContents::Eol | TokenContents::Semicolon => {
                    ends_with_pipe &= token.contents == TokenContents::Eol;
                    tokens.next_token();
                    break;
                }
                TokenContents::Eof => break,
                _ => {
                    ends_with_pipe = token.contents == TokenContents::Pipe;
                    attribute_line.push(Token { contents: TokenContents::Item, span: token.span });
                }
            }
            tokens.next_token();
        }
        lite_command.attributes.push(attribute_line);
        // A line ending with `|` keeps nu's command open across comment lines
        // (`last_non_comment_token`), so the definition may follow them.
        while ends_with_pipe
            && let [comment, eol, ..] = tokens.remaining()
            && comment.contents == TokenContents::Comment
            && eol.contents == TokenContents::Eol
        {
            working_set.add_comment(comment.span);
            lite_command.comments.push(Comment { span: comment.span });
            tokens.next_token();
            tokens.next_token();
        }
        if let Some(token) = tokens.peek_token()
            && matches!(token.contents, TokenContents::Eol | TokenContents::Comment)
        {
            return Err(cut(Diagnostic::message("attributes must be followed by a definition", token.span)
                .with_help("put the `def`, `extern` or `export` on the line right after the attributes")));
        }
    }
    Ok(())
}

/// nu's lite parse of the tokens inside `[...]`: the items of each
/// `|`-separated command. A redirection and its target are dropped from the
/// items as nu drops them (`[a o> b]` is `[a]`) and recorded as ignored text;
/// a redirection with nothing before it, a missing target, a second
/// redirection of the same stream, `||` and a `|` at the end are errors.
/// After an assignment operator everything is an item (`[a = b | c]` has
/// five). `;` is left to the caller, which has already refused it.
pub fn lite_parse_parts(working_set: &WorkingSet<'_>, tokens: &[Token]) -> ParseResult<Vec<Vec<Token>>> {
    #[derive(Clone, Copy)]
    enum Redirected {
        None,
        Single(RedirectionSource),
        Separate,
    }
    if let Some(last) = tokens.last().filter(|token| token.contents == TokenContents::Pipe) {
        return Err(cut(Diagnostic::new(ErrorKind::UnexpectedEof("list item after `|`"), last.span)));
    }
    let mut groups: Vec<Vec<Token>> = Vec::new();
    let mut parts: Vec<Token> = Vec::new();
    let mut redirected = Redirected::None;
    let mut assignment = false;
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        index += 1;
        match token.contents {
            TokenContents::AssignmentOperator(_) => {
                assignment = true;
                parts.push(*token);
            }
            _ if assignment => parts.push(*token),
            TokenContents::Item => parts.push(*token),
            TokenContents::PipePipe => {
                return Err(cut(Diagnostic::new(
                    ErrorKind::ShellSyntax { found: "||", use_instead: "or" },
                    token.span,
                )));
            }
            TokenContents::Pipe => {
                groups.push(std::mem::take(&mut parts));
                redirected = Redirected::None;
            }
            TokenContents::Redirection(operator) => {
                if parts.is_empty() {
                    return Err(cut(Diagnostic::message("unexpected redirection: nothing to redirect", token.span)));
                }
                redirected = match (redirected, operator.source()) {
                    (Redirected::None, source) => Redirected::Single(source),
                    (Redirected::Single(RedirectionSource::Stdout), RedirectionSource::Stderr)
                    | (Redirected::Single(RedirectionSource::Stderr), RedirectionSource::Stdout) => {
                        Redirected::Separate
                    }
                    _ => {
                        return Err(cut(Diagnostic::message("multiple redirections of the same stream", token.span)));
                    }
                };
                working_set.add_ignored(token.span);
                if operator.is_pipe() {
                    groups.push(std::mem::take(&mut parts));
                    redirected = Redirected::None;
                    continue;
                }
                match tokens.get(index) {
                    Some(target) if target.contents == TokenContents::Item => {
                        working_set.add_ignored(target.span);
                        index += 1;
                    }
                    _ => return Err(cut(Diagnostic::expected("redirection target", token.span.past()))),
                }
            }
            TokenContents::Comment | TokenContents::Eol | TokenContents::Semicolon | TokenContents::Eof => {}
        }
    }
    groups.push(parts);
    Ok(groups)
}
