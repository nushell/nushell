//! The lite parse: grouping a block's tokens into pipelines of commands, once for each block
//! (nu-parser's `lite_parser.rs`). [`LiteBlock`] is what the block's predeclaration, its statements
//! and its first lite-parse error all read.
//!
//! It decides where commands and pipelines end, which comments a command keeps, what an assignment
//! takes with it, and which items are attribute lines and redirections. It never looks inside an
//! item. Its loop is nu's `lite_parse`, so nu's rules hold without being stated here again, with one
//! step before them: nu's lexer drops an end of line that only comment lines stand between and a
//! `|` starting a later line, and the winnow lexer keeps them.

use crate::winnow::syntax::ast::LiteError;
use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{ParseResult, cut};
use crate::winnow::syntax::lex::{RedirectionOperator, RedirectionSource, Token, TokenContents};
use crate::winnow::syntax::span::{Span, Spanned};

use super::WorkingSet;
use super::tokens::Tokens;

/// The diagnostic of a lite-parse error, for the parse's verdict (the consumer reports nu's).
pub fn lite_diagnostic(error: LiteError) -> Diagnostic {
    match error {
        LiteError::ShellOrOr(span) => Diagnostic::new(
            ErrorKind::ShellSyntax {
                found: "||",
                use_instead: "or",
            },
            span,
        )
        .with_help("use `or` for boolean logic, or `try { } catch { }` to run a fallback command"),
        LiteError::PipelineMissingEnd(span) => {
            Diagnostic::new(ErrorKind::UnexpectedEof("command after `|`"), span)
                .with_context("pipeline")
                .with_help("the pipeline has no end: add a command after the `|` or remove it")
        }
        LiteError::MissingRedirectionTarget(span) => {
            Diagnostic::expected("redirection target", span)
        }
        LiteError::NothingToRedirect(span) => {
            Diagnostic::message("unexpected redirection: nothing to redirect", span)
        }
        LiteError::MultipleRedirections { first, second, .. } => {
            Diagnostic::message("multiple redirections of the same stream", second)
                .with_help(format!("the stream is already redirected at {first}"))
        }
    }
}

/// The last token that is not part of a trailing `([Comment]+ [Eol])*`
/// sequence: nu's `last_non_comment_token`, used to tell `ls |\n` (fine)
/// from `ls |` and `ls |\n# c` (a pipeline with no end).
pub fn last_non_comment_token(tokens: &[Token]) -> Option<TokenContents> {
    let mut expect = TokenContents::Comment;
    for token in tokens.iter().rev() {
        match (token.contents, expect) {
            (TokenContents::Comment, TokenContents::Comment | TokenContents::Eol) => {
                expect = TokenContents::Eol
            }
            (TokenContents::Eol, TokenContents::Eol) => expect = TokenContents::Comment,
            (kind, _) => return Some(kind),
        }
    }
    None
}

/// The parts of a statement nu reads by its first word, what follows an assignment operator among
/// them items of its call (`def = [] {}` defines the command `=`, and `use = | x` looks for a module
/// named `=`): nu's lite parser takes them all as the command's parts.
pub(super) fn absorbed_as_items(tokens: &[Token]) -> Vec<Token> {
    tokens.iter().copied().map(as_item).collect()
}

/// `token` as an item: a word of the command.
pub(super) fn as_item(token: Token) -> Token {
    Token {
        contents: TokenContents::Item,
        span: token.span,
    }
}

/// nu's lite parse of the tokens inside `[...]`: the items of each
/// `|`-separated command. A redirection and its target are dropped from the
/// items as nu drops them (`[a o> b]` is `[a]`) and recorded as ignored text;
/// a redirection with nothing before it, a missing target, a second
/// redirection of the same stream, `||` and a `|` at the end are errors.
/// After an assignment operator everything is an item (`[a = b | c]` has
/// five). `;` is left to the caller, which has already refused it.
pub fn lite_parse_parts(
    working_set: &WorkingSet<'_>,
    tokens: &[Token],
) -> ParseResult<Vec<Vec<Token>>> {
    /// What the current command has redirected, to refuse a second redirection of a stream.
    #[derive(Clone, Copy)]
    enum Redirected {
        /// Nothing yet.
        None,
        /// One redirection, of this source (`o+e>` counts as both streams).
        Single(RedirectionSource),
        /// Stdout and stderr, each to its own target: nothing more may follow.
        Separate,
    }
    if let Some(last) = tokens
        .last()
        .filter(|token| token.contents == TokenContents::Pipe)
    {
        return Err(cut(Diagnostic::new(
            ErrorKind::UnexpectedEof("list item after `|`"),
            last.span,
        )));
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
                    ErrorKind::ShellSyntax {
                        found: "||",
                        use_instead: "or",
                    },
                    token.span,
                )));
            }
            TokenContents::Pipe => {
                groups.push(std::mem::take(&mut parts));
                redirected = Redirected::None;
            }
            TokenContents::Redirection(operator) => {
                if parts.is_empty() {
                    return Err(cut(Diagnostic::message(
                        "unexpected redirection: nothing to redirect",
                        token.span,
                    )));
                }
                redirected = match (redirected, operator.source()) {
                    (Redirected::None, source) => Redirected::Single(source),
                    (Redirected::Single(RedirectionSource::Stdout), RedirectionSource::Stderr)
                    | (Redirected::Single(RedirectionSource::Stderr), RedirectionSource::Stdout) => {
                        Redirected::Separate
                    }
                    _ => {
                        return Err(cut(Diagnostic::message(
                            "multiple redirections of the same stream",
                            token.span,
                        )));
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
                    _ => {
                        return Err(cut(Diagnostic::expected(
                            "redirection target",
                            token.span.past(),
                        )));
                    }
                }
            }
            TokenContents::Comment
            | TokenContents::Eol
            | TokenContents::Semicolon
            | TokenContents::Eof => {}
        }
    }
    groups.push(parts);
    Ok(groups)
}

/// One command of a [`LiteBlock`] (nu's `LiteCommand`). Its lists are ranges of the block's, read
/// through [`LiteBlock::parts`] and the others.
#[derive(Clone, Copy, Debug)]
pub struct LiteBlockCommand {
    parts_start: u32,
    parts_end: u32,
    /// Where its parts after its attribute lines start ([`LiteBlock::command_parts`]).
    command_start: u32,
    attributes_start: u32,
    attributes_end: u32,
    comments_start: u32,
    comments_end: u32,
    redirections_start: u32,
    redirections_end: u32,
    /// What nu's `command.pipe` holds: the `|` after it, or one that no command took before it.
    pub pipe: Option<Span>,
    /// The `|` no command took before it, which nu's `command.pipe` held until a `|` after it
    /// took its place.
    pub pipe_before: Option<Span>,
    /// The index in the block's tokens of what ended it: a `|`, an `e>|`, an end of line, a `;`,
    /// or one past the last token.
    pub ended_at: u32,
    /// Whether it is the last command of its pipeline.
    ends_pipeline: bool,
}

/// One command as [`LiteBlock::command`] gives it to the statement reader.
#[derive(Debug)]
pub struct LiteCommand<'b> {
    /// Its parts after its attribute lines.
    pub parts: &'b [Token],
    /// Its attribute lines' parts, a slice for each line.
    pub attributes: Vec<&'b [Token]>,
    /// Its redirections, in the order they are written.
    pub redirections: &'b [LiteRedirection],
    /// Byte offset just past its last part, or, with none, where it was ended.
    pub end: usize,
}

impl<'b> LiteCommand<'b> {
    /// A stream over its parts.
    pub fn tokens<'t, 'a>(&self, working_set: &'t WorkingSet<'a>) -> Tokens<'t, 'a>
    where
        'b: 't,
    {
        Tokens::new(working_set, self.parts, self.end)
    }
}

/// A redirection of a command: its operator, and its file (none for `e>|` and `o+e>|`).
pub type LiteRedirection = (Spanned<RedirectionOperator>, Option<Token>);

/// A block's tokens grouped as nu's `lite_parse` groups them, each kind of list held once for the
/// whole block: reading a block costs a few allocations, not a few per command.
#[derive(Debug, Default)]
pub struct LiteBlock {
    parts: Vec<Token>,
    commands: Vec<LiteBlockCommand>,
    comments: Vec<Span>,
    /// One past the end of each attribute line's parts, in `parts`.
    attribute_ends: Vec<u32>,
    redirections: Vec<LiteRedirection>,
    /// Every lite-parse error in the order nu's loop meets them, each with the index of the command
    /// being read then (nu keeps the first; a standalone parse reports them all).
    errors: Vec<(u32, LiteError)>,
    /// A `|` ending the block, which nu reports in place of the block's first error.
    missing_end: Option<LiteError>,
}

/// Where the lists of the command being read start. It stays as it is until the command has parts
/// or a redirection, as nu keeps a command it does not take: its comments and its pipe go on to the
/// next one.
#[derive(Default)]
struct CommandStart {
    parts: usize,
    attributes: usize,
    comments: usize,
    redirections: usize,
    pipe: Option<Span>,
}

/// What the next tokens of a block are read as (nu's `Mode`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum LiteMode {
    /// An assignment takes pipes and redirections up to the end of the line or a `;`.
    Assignment,
    /// An attribute line ends at the end of the line or a `;`.
    Attribute,
    Normal,
}

impl LiteBlock {
    /// `tokens`, a block's from the winnow lexer without its `Eof`, lite-parsed. nu's lexer drops
    /// an end of line that only comment lines stand between and a `|` starting a later line, which
    /// the winnow lexer keeps, so this skips those.
    pub fn parse(source: &str, tokens: &[Token]) -> Self {
        Self::read(source, tokens, true)
    }

    /// [`Self::parse`] for `tokens` with those ends of line dropped already, as nu's lexer gives
    /// them.
    pub fn parse_folded(source: &str, tokens: &[Token]) -> Self {
        Self::read(source, tokens, false)
    }

    /// The first error nu's lite parser reports: a `|` ending the block, else the block's first.
    pub fn error(&self) -> Option<LiteError> {
        self.missing_end
            .or_else(|| self.errors.first().map(|(_, error)| *error))
    }

    /// A `|` ending the block, which has no pipeline of its own to report it.
    pub fn missing_end(&self) -> Option<LiteError> {
        self.missing_end
    }

    /// The first error after the block's last command: of a command nu drops, with no later one
    /// to carry it (an `e>|` with nothing before it).
    pub fn dropped_error(&self) -> Option<LiteError> {
        self.errors_from(self.commands.len()).next()
    }

    /// The errors met while reading the commands from `start` up to `end`.
    pub fn errors_in(&self, start: usize, end: usize) -> impl Iterator<Item = LiteError> + '_ {
        // In the order of their commands: a block with many stays linear.
        let from = self
            .errors
            .partition_point(|(at, _)| (*at as usize) < start);
        let to = self.errors.partition_point(|(at, _)| (*at as usize) < end);
        self.errors[from..to].iter().map(|(_, error)| *error)
    }

    /// The errors met from the command `start` on.
    pub fn errors_from(&self, start: usize) -> impl Iterator<Item = LiteError> + '_ {
        self.errors_in(start, usize::MAX)
    }

    /// The pipelines, each of its commands.
    pub fn pipelines(&self) -> impl Iterator<Item = &[LiteBlockCommand]> {
        self.commands
            .split_inclusive(|command| command.ends_pipeline)
    }

    /// The commands from `start` on, as [`Self::pipelines`] groups them.
    pub fn pipelines_from(&self, start: usize) -> impl Iterator<Item = &[LiteBlockCommand]> {
        self.commands
            .get(start..)
            .unwrap_or_default()
            .split_inclusive(|command| command.ends_pipeline)
    }

    /// How many commands the block has.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// The command at `index`.
    pub fn command_at(&self, index: usize) -> Option<&LiteBlockCommand> {
        self.commands.get(index)
    }

    /// `command`, with `tokens` the block's, read for the statement reader.
    pub fn command<'b>(&'b self, command: &LiteBlockCommand, tokens: &[Token]) -> LiteCommand<'b> {
        let parts = self.command_parts(command);
        let end = parts.last().map_or_else(
            || {
                tokens
                    .get(command.ended_at as usize)
                    .or(tokens.last())
                    .map_or(0, |token| token.span.start)
            },
            |last| last.span.end,
        );
        let mut line_start = command.parts_start as usize;
        let attributes = self.attribute_ends
            [command.attributes_start as usize..command.attributes_end as usize]
            .iter()
            .map(|end| {
                let line = &self.parts[line_start..*end as usize];
                line_start = *end as usize;
                line
            })
            .collect();
        LiteCommand {
            parts,
            attributes,
            redirections: self.redirections(command),
            end,
        }
    }

    /// A command's parts, those of its attribute lines included (nu's `parts`).
    pub fn parts(&self, command: &LiteBlockCommand) -> &[Token] {
        &self.parts[command.parts_start as usize..command.parts_end as usize]
    }

    /// A command's parts after its attribute lines (nu's `command_parts`).
    pub fn command_parts(&self, command: &LiteBlockCommand) -> &[Token] {
        &self.parts[command.command_start as usize..command.parts_end as usize]
    }

    /// A command's comments (nu's `comments`).
    pub fn comments(&self, command: &LiteBlockCommand) -> &[Span] {
        &self.comments[command.comments_start as usize..command.comments_end as usize]
    }

    /// One past the end of each of a command's attribute lines, counted in [`Self::parts`] (nu's
    /// `attribute_idx`).
    pub fn attribute_ends<'b>(
        &'b self,
        command: &'b LiteBlockCommand,
    ) -> impl Iterator<Item = usize> + 'b {
        self.attribute_ends[command.attributes_start as usize..command.attributes_end as usize]
            .iter()
            .map(|end| (end - command.parts_start) as usize)
    }

    /// A command's redirections, in the order they are written.
    pub fn redirections(&self, command: &LiteBlockCommand) -> &[LiteRedirection] {
        &self.redirections[command.redirections_start as usize..command.redirections_end as usize]
    }

    /// nu's `lite_parse`.
    fn read(source: &str, tokens: &[Token], fold: bool) -> Self {
        let mut block = Self::default();
        if tokens.is_empty() {
            return block;
        }
        // No more parts than tokens: an operator pushed as a part was no part when it was read.
        block.parts.reserve_exact(tokens.len());
        let source = source.as_bytes();
        let mut command = CommandStart::default();
        let mut last = TokenContents::Eol;
        let mut file_redirection: Option<Spanned<RedirectionOperator>> = None;
        // The comment lines before the next command (nu's `curr_comment`).
        let mut pending: Vec<Span> = Vec::new();
        let mut mode = LiteMode::Normal;
        // The ends of line before `fold_until` share one answer: whether a `|` follows their run
        // of comment lines. A long run of comment lines is walked once.
        let mut fold_until = 0;
        let mut folding = false;
        for (index, &token) in tokens.iter().enumerate() {
            let span = token.span;
            if fold && token.contents == TokenContents::Eol {
                if index >= fold_until {
                    fold_until = after_comment_lines(tokens, index + 1);
                    // A `&&` is a `Pipe` too, which nu's lexer reads apart and folds nothing for.
                    folding = tokens.get(fold_until).is_some_and(|next| {
                        next.contents == TokenContents::Pipe && next.span.len() == 1
                    });
                }
                if folding {
                    continue;
                }
            }
            match mode {
                LiteMode::Attribute => match token.contents {
                    TokenContents::Eol | TokenContents::Semicolon => {
                        block.attribute_ends.push(block.parts.len() as u32);
                        mode = LiteMode::Normal;
                        if matches!(last, TokenContents::Eol | TokenContents::Semicolon) {
                            pending.clear();
                            block.end_command(&mut command, index);
                            block.end_pipeline();
                        }
                    }
                    TokenContents::Comment => {
                        block.comments.push(span);
                        pending.clear();
                    }
                    // Everything on the line is a word of the attribute, pipes included.
                    _ => block.parts.push(as_item(token)),
                },
                LiteMode::Assignment => match token.contents {
                    TokenContents::Eol => {
                        if !ends_after_pipe(tokens, index, last) {
                            mode = LiteMode::Normal;
                            block.end_command(&mut command, index);
                            block.end_pipeline();
                        }
                    }
                    TokenContents::Semicolon => {
                        mode = LiteMode::Normal;
                        block.end_command(&mut command, index);
                        block.end_pipeline();
                    }
                    // No comment lines wait here: the operator took them when it was read.
                    TokenContents::Comment => block.comments.push(span),
                    _ => block.parts.push(token),
                },
                LiteMode::Normal => match file_redirection.take() {
                    Some(operator) => block.after_file_redirection(
                        &mut command,
                        &mut pending,
                        operator,
                        token,
                        index,
                    ),
                    None => match token.contents {
                        TokenContents::PipePipe => {
                            block.report(LiteError::ShellOrOr(span));
                            block.parts.push(as_item(token));
                        }
                        TokenContents::Item => {
                            if source.get(span.start) == Some(&b'@') {
                                if matches!(last, TokenContents::Eol | TokenContents::Semicolon) {
                                    mode = LiteMode::Attribute;
                                }
                            } else {
                                block.attach_comments(&command, &mut pending);
                            }
                            block.parts.push(token);
                        }
                        TokenContents::AssignmentOperator(_) => {
                            mode = LiteMode::Assignment;
                            block.attach_comments(&command, &mut pending);
                            block.parts.push(token);
                        }
                        TokenContents::Redirection(operator) if operator.is_pipe() => {
                            block.add_redirection(&command, Spanned::new(operator, span), None);
                            block.end_command_at_pipe(&mut command, span, index);
                        }
                        TokenContents::Redirection(operator) => {
                            if block.parts.len() == command.parts {
                                block.report(LiteError::NothingToRedirect(span));
                            }
                            file_redirection = Some(Spanned::new(operator, span));
                        }
                        TokenContents::Pipe => {
                            block.end_command_at_pipe(&mut command, span, index);
                        }
                        TokenContents::Eol => {
                            if !ends_after_pipe(tokens, index, last) {
                                block.end_command(&mut command, index);
                                block.end_pipeline();
                            }
                            if last == TokenContents::Eol {
                                pending.clear();
                            }
                        }
                        TokenContents::Semicolon => {
                            block.end_command(&mut command, index);
                            block.end_pipeline();
                        }
                        // A comment beside something, or one before it.
                        TokenContents::Comment => {
                            if last == TokenContents::Eol {
                                pending.push(span);
                            } else {
                                block.comments.push(span);
                                pending.clear();
                            }
                        }
                        TokenContents::Eof => {}
                    },
                },
            }
            last = token.contents;
        }
        if let Some(operator) = file_redirection {
            block.parts.push(operator_item(operator.span));
            block.report(LiteError::MissingRedirectionTarget(operator.span));
        }
        if mode == LiteMode::Attribute {
            block.attribute_ends.push(block.parts.len() as u32);
        }
        block.end_command(&mut command, tokens.len());
        block.end_pipeline();
        if last_non_comment_token(tokens) == Some(TokenContents::Pipe)
            && let Some(last) = tokens.last()
        {
            block.missing_end = Some(LiteError::PipelineMissingEnd(last.span));
        }
        block
    }

    /// Note `error`, met while reading the current command. nu's redirection reports the same
    /// nothing-to-redirect twice, when it meets the operator and when it meets the file.
    fn report(&mut self, error: LiteError) {
        let entry = (self.commands.len() as u32, error);
        if self.errors.last() != Some(&entry) {
            self.errors.push(entry);
        }
    }

    /// `token` after the file redirection `operator`: its file, or what nu reads there instead.
    fn after_file_redirection(
        &mut self,
        command: &mut CommandStart,
        pending: &mut Vec<Span>,
        operator: Spanned<RedirectionOperator>,
        token: Token,
        index: usize,
    ) {
        let span = token.span;
        let operator_part = operator_item(operator.span);
        match token.contents {
            TokenContents::PipePipe => {
                self.report(LiteError::ShellOrOr(span));
                self.parts.extend([operator_part, as_item(token)]);
            }
            TokenContents::Item => {
                if !self.add_redirection(command, operator, Some(token)) {
                    self.parts.extend([operator_part, token]);
                }
            }
            TokenContents::AssignmentOperator(_) => {
                self.report(LiteError::MissingRedirectionTarget(span));
                self.parts.extend([operator_part, token]);
            }
            TokenContents::Redirection(next) if !next.is_pipe() => {
                self.report(LiteError::MissingRedirectionTarget(span));
                self.parts.extend([operator_part, as_item(token)]);
            }
            TokenContents::Pipe | TokenContents::Redirection(_) => {
                self.report(LiteError::MissingRedirectionTarget(span));
                self.parts.push(operator_part);
                self.end_command_at_pipe(command, span, index);
            }
            TokenContents::Eol => {
                self.report(LiteError::MissingRedirectionTarget(span));
                self.parts.push(operator_part);
                self.end_command(command, index);
            }
            TokenContents::Semicolon => {
                self.report(LiteError::MissingRedirectionTarget(span));
                self.parts.push(operator_part);
                self.end_command(command, index);
                self.end_pipeline();
            }
            TokenContents::Comment => {
                self.report(LiteError::MissingRedirectionTarget(operator.span));
                self.parts.push(operator_part);
                self.comments.push(span);
                pending.clear();
            }
            TokenContents::Eof => {}
        }
    }

    /// The comment lines before the command replace its comments (nu's `command.comments =
    /// curr_comment`), once it has a part that is no attribute.
    fn attach_comments(&mut self, command: &CommandStart, pending: &mut Vec<Span>) {
        if !pending.is_empty() {
            self.comments.truncate(command.comments);
            self.comments.append(pending);
        }
    }

    /// Add a redirection to the command being read (nu's `try_add_redirection`), reporting one it
    /// refuses: with nothing to redirect, or of a stream redirected already, which leaves the
    /// command with none. Returns whether it was added.
    fn add_redirection(
        &mut self,
        command: &CommandStart,
        operator: Spanned<RedirectionOperator>,
        file: Option<Token>,
    ) -> bool {
        let source = operator.item.source();
        let refused = match &self.redirections[command.redirections..] {
            [] if self.parts.len() == command.parts => {
                Some(LiteError::NothingToRedirect(operator.span))
            }
            [] => None,
            [(first, _)] => match (first.item.source(), source) {
                (RedirectionSource::Stdout, RedirectionSource::Stderr)
                | (RedirectionSource::Stderr, RedirectionSource::Stdout) => None,
                (first_source, _) => Some(LiteError::MultipleRedirections {
                    source: first_source,
                    first: first.span,
                    second: operator.span,
                }),
            },
            // Stdout and stderr, each to its own: `o+e>` meets the stdout one.
            separate => {
                let met = match source {
                    RedirectionSource::Stderr => RedirectionSource::Stderr,
                    _ => RedirectionSource::Stdout,
                };
                separate
                    .iter()
                    .find(|(first, _)| first.item.source() == met)
                    .map(|(first, _)| LiteError::MultipleRedirections {
                        source: met,
                        first: first.span,
                        second: operator.span,
                    })
            }
        };
        match refused {
            Some(error) => {
                self.report(error);
                self.redirections.truncate(command.redirections);
                false
            }
            None => {
                self.redirections.push((operator, file));
                true
            }
        }
    }

    /// nu's `command.pipe = pipe; pipeline.push(&mut command)`: the `|` at `at` ends the command
    /// read so far, if there is one, or stays with the one to come.
    fn end_command_at_pipe(&mut self, command: &mut CommandStart, pipe: Span, at: usize) {
        let pipe_before = command.pipe.replace(pipe);
        if self.has_command(command) {
            self.push_command(command, pipe_before, at);
        }
    }

    /// nu's `pipeline.push(&mut command)`, at the token at `at`: the command read so far is one if
    /// it has parts or a redirection, and stays open otherwise.
    fn end_command(&mut self, command: &mut CommandStart, at: usize) {
        if self.has_command(command) {
            self.push_command(command, command.pipe, at);
        }
    }

    fn has_command(&self, command: &CommandStart) -> bool {
        self.parts.len() != command.parts || self.redirections.len() != command.redirections
    }

    fn push_command(
        &mut self,
        command: &mut CommandStart,
        pipe_before: Option<Span>,
        ended_at: usize,
    ) {
        let has_attributes = self.attribute_ends.len() > command.attributes;
        let command_start = match self.attribute_ends.last() {
            Some(&end) if has_attributes => end,
            _ => command.parts as u32,
        };
        self.commands.push(LiteBlockCommand {
            parts_start: command.parts as u32,
            parts_end: self.parts.len() as u32,
            command_start,
            attributes_start: command.attributes as u32,
            attributes_end: self.attribute_ends.len() as u32,
            comments_start: command.comments as u32,
            comments_end: self.comments.len() as u32,
            redirections_start: command.redirections as u32,
            redirections_end: self.redirections.len() as u32,
            pipe: command.pipe.take(),
            pipe_before,
            ended_at: ended_at as u32,
            ends_pipeline: false,
        });
        command.parts = self.parts.len();
        command.attributes = self.attribute_ends.len();
        command.comments = self.comments.len();
        command.redirections = self.redirections.len();
    }

    /// nu's `block.push(&mut pipeline)`: the commands since the last one are a pipeline, if any.
    fn end_pipeline(&mut self) {
        if let Some(last) = self.commands.last_mut() {
            last.ends_pipeline = true;
        }
    }
}

/// The index in `tokens` after the comment lines from `at` on (`(Comment Eol)*`).
fn after_comment_lines(tokens: &[Token], mut at: usize) -> usize {
    while let [comment, eol, ..] = tokens.get(at..).unwrap_or_default()
        && comment.contents == TokenContents::Comment
        && eol.contents == TokenContents::Eol
    {
        at += 2;
    }
    at
}

/// Whether the end of line at `index`, with `last` the token before it, leaves a `|` as the last
/// token, comment lines skipped: the pipeline goes on (nu's `last_non_comment_token`). No end of
/// line it walks past was dropped: one that was has a `|` after it before this one.
fn ends_after_pipe(tokens: &[Token], index: usize, last: TokenContents) -> bool {
    match last {
        TokenContents::Pipe => true,
        TokenContents::Comment | TokenContents::Eol => {
            last_non_comment_token(&tokens[..index]) == Some(TokenContents::Pipe)
        }
        _ => false,
    }
}

/// A redirection's operator as a word of the command, which nu makes it when it refuses it.
fn operator_item(span: Span) -> Token {
    Token {
        contents: TokenContents::Item,
        span,
    }
}
