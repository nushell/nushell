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

use super::lite_parser::{
    AfterPipe, LiteCommand, after_pipe, last_non_comment_token, lite_diagnostic,
    parse_lite_command, pipe_on_a_later_line, skip_to_statement_end, take_pipe_on_later_line,
};
use super::parse_calls::{keyword_signature_of_call, parse_attribute};
use super::parse_def::{
    Definitions, PredeclaredDef, TypeAliasPredecl, parse_def_predecl, scan_predecls,
};
use super::parse_expressions::{
    ExpectedShape, Position, is_env_variable_name, parse_builtin_commands, parse_expression,
    parse_value,
};
use super::parse_helpers::garbage_pipeline;
use super::tokens::Tokens;
use super::working_set::{CommandLookup, ParseState};
use super::{ParseConfig, WorkingSet};

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
pub fn parse_block_lexed_from<'a>(
    mut tokens: Tokens<'_, 'a>,
    span: Span,
    lexed: Span,
) -> Block<'a> {
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
    let missing_end = missing_end(tokens.all());
    if let Some(error) = &lex_error {
        working_set.error(error.diagnostic().as_recovered());
    }
    if let Some(error) = missing_end {
        working_set.error(lite_diagnostic(error).as_recovered());
    }
    parse_def_predecl(working_set, tokens.all());
    let mut pipelines: Vec<Pipeline<'a>> = Vec::new();
    let mut state = StatementsState::default();
    parse_statements(&mut tokens, &mut state, &mut pipelines);
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
        without_pipelines.lite_error = without_pipelines.lite_error.or(state.dropped_error);
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
    /// The next token to parse.
    position: usize,
    /// Where the statement loop stood when the last call returned: the comments waiting for
    /// the next statement, a `|` carried over to it.
    statements: StatementsState,
    /// The block's lite-parse error, which nu reports before its statements' errors
    /// ([`BlockStatements::lite_error`]).
    lite_error: Option<LiteError>,
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
        let definitions = scan_predecls(&working_set, stream.all());
        let lite_error = missing_end(stream.all()).or_else(|| first_lite_error(source, &tokens));
        // What nu goes on past (a definition declared twice) the consumer reports.
        let block_errors = working_set.take_errors_from(0);
        if block_errors.iter().any(|diagnostic| !diagnostic.recovered) {
            return Err(block_errors);
        }
        let parse_state = working_set.into_state();
        Ok((
            Self {
                tokens,
                position: 0,
                statements: StatementsState::default(),
                lite_error,
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
    /// A statement that fails to parse is handed over without the `;` after it and a comment
    /// after that `;`: `parse_block` adds them to its garbage pipeline once it meets them, and
    /// a streamed pipeline is the sink's by then.
    pub fn parse<'a>(
        &mut self,
        source: &'a str,
        lookup: impl CommandLookup + 'a,
        sink: &mut dyn FnMut(Pipeline<'a>, Vec<Diagnostic>, usize) -> bool,
    ) {
        let parse_state = std::mem::take(&mut self.parse_state);
        let working_set = WorkingSet::with_state(source, lookup, parse_state);
        let mut tokens = Tokens::from_lexed(&working_set, &self.tokens);
        tokens.reset_to(self.position);
        parse_statements(
            &mut tokens,
            &mut self.statements,
            &mut StreamingSink {
                sink,
                working_set: &working_set,
            },
        );
        self.position = tokens.position();
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
    /// ([`LiteError::PipelineMissingEnd`]), else its first command's ([`first_lite_error`]).
    pub fn lite_error(&self) -> Option<LiteError> {
        self.lite_error
    }

    /// The lite-parse error of a command nu dropped so far, which no statement carries (an
    /// `e>|` with nothing before it): the block's, unless a statement has one.
    pub fn dropped_error(&self) -> Option<LiteError> {
        self.statements.dropped_error
    }

    /// The block's first lexer error, which nu reports before its statements' errors and goes
    /// on past; the text it is in starts where the block does.
    pub fn lex_error(&self) -> Option<&LexError> {
        self.lex_error.as_ref()
    }

    /// Whether every statement was parsed.
    pub fn is_done(&self) -> bool {
        self.position >= self.stream_len()
    }

    /// The text of the statements still to parse: from the next token to the end of the block.
    pub fn remaining(&self) -> Span {
        let end = self.tokens.last().map_or(0, |token| token.span.end);
        let start = self
            .tokens
            .get(self.position)
            .map_or(end, |token| token.span.start);
        Span::new(start, end)
    }

    /// The first two words of the next statement (the second when it is on the same line),
    /// to tell what kind of statement comes without parsing it.
    pub fn next_words<'a>(&self, source: &'a str) -> (Option<&'a str>, Option<&'a str>) {
        let rest = &self.tokens[self.position.min(self.stream_len())..self.stream_len()];
        let mut words = rest
            .iter()
            .skip_while(|token| {
                matches!(
                    token.contents,
                    TokenContents::Eol | TokenContents::Comment | TokenContents::Semicolon
                )
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

/// nu's lite parser: a block whose last token, skipping trailing comment lines, is a `|` has a
/// pipeline with no end (`ls |`, `ls |\n# c`, `alias x = ls |`), whatever absorbed the pipe,
/// reported at the block's last token in place of the block's first lite-parse error
/// ([`LiteError::PipelineMissingEnd`]).
fn missing_end(tokens: &[Token]) -> Option<LiteError> {
    if last_non_comment_token(tokens) != Some(TokenContents::Pipe) {
        return None;
    }
    let last = tokens
        .iter()
        .rev()
        .find(|token| token.contents != TokenContents::Eof)?;
    Some(LiteError::PipelineMissingEnd(last.span))
}

/// The first lite-parse error of the commands of a block lexed into `tokens`, found by the
/// lite parse alone ([`parse_pipeline`] with `lite_only`), as nu's `lite_parse` finds it before
/// any statement is parsed: the statements parsed before the one it is in see it reported (a
/// definition's body is not compiled then). Only a `||` or a redirection makes one, so a block
/// with neither is not scanned. The scan stops at a statement whose lite parse fails, whose
/// error, and those after, the statements report as they are parsed.
fn first_lite_error(source: &str, tokens: &[Token]) -> Option<LiteError> {
    if !tokens.iter().any(|token| {
        matches!(
            token.contents,
            TokenContents::PipePipe | TokenContents::Redirection(_)
        )
    }) {
        return None;
    }
    // A working set of its own: the diagnostics and comments the scan records are not the
    // block's.
    let working_set = WorkingSet::new(source, &ParseConfig::empty());
    let mut stream = Tokens::from_lexed(&working_set, tokens);
    let mut carried_comments = Vec::new();
    let mut carried_pipe = CarriedPipes::default();
    while let Some(token) = stream.peek_token() {
        if matches!(
            token.contents,
            TokenContents::Eol | TokenContents::Semicolon | TokenContents::Comment
        ) {
            stream.next_token();
            continue;
        }
        let mut lite_error = None;
        parse_pipeline(
            &mut stream,
            &mut Vec::new(),
            &mut carried_comments,
            &mut carried_pipe,
            &mut None,
            &mut lite_error,
            true,
        )
        .ok()?;
        if lite_error.is_some() {
            return lite_error;
        }
    }
    None
}

/// Where [`parse_statements`] puts the pipelines it parses: a `Vec` for [`parse_block`],
/// which leaves every diagnostic in the working set for the whole parse to collect, or a
/// [`StreamingSink`] for [`BlockStatements::parse`], which hands each statement over with
/// its own diagnostics.
trait StatementSink<'a> {
    /// Take the next pipeline. `before` is how the working set stood just before it was parsed.
    /// Returns whether to go on.
    fn push(&mut self, pipeline: Pipeline<'a>, before: StatementStart) -> bool;
    /// Take the placeholder for a statement covering `span` that failed to
    /// parse, with the lite-parse error found before it failed. Returns whether to go on.
    fn push_garbage(
        &mut self,
        span: Span,
        leading_comments: Vec<Comment>,
        lite_error: Option<LiteError>,
        before: StatementStart,
    ) -> bool;
    /// The pipeline pushed last, while it can still be amended (its
    /// terminator, a comment after it on its line); `None` once it was handed over.
    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>>;
}

/// The whole block at once ([`parse_block`]): the diagnostics stay in the working set.
impl<'a> StatementSink<'a> for Vec<Pipeline<'a>> {
    fn push(&mut self, pipeline: Pipeline<'a>, _before: StatementStart) -> bool {
        Vec::push(self, pipeline);
        true
    }

    fn push_garbage(
        &mut self,
        span: Span,
        _leading_comments: Vec<Comment>,
        lite_error: Option<LiteError>,
        _before: StatementStart,
    ) -> bool {
        let mut garbage = garbage_pipeline(span);
        garbage.lite_error = lite_error.map(Box::new);
        Vec::push(self, garbage);
        true
    }

    fn last_mut(&mut self) -> Option<&mut Pipeline<'a>> {
        self.as_mut_slice().last_mut()
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

    fn push_garbage(
        &mut self,
        span: Span,
        leading_comments: Vec<Comment>,
        lite_error: Option<LiteError>,
        before: StatementStart,
    ) -> bool {
        // The statement's doc comments go with it, for whoever parses it again.
        let mut garbage = garbage_pipeline(span);
        garbage.leading_comments = leading_comments;
        garbage.lite_error = lite_error.map(Box::new);
        self.push(garbage, before)
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
    /// Comments after a `;`, or after a `|` that ends its pipeline, on that line: nu's lite
    /// parser puts them in the next command, which it starts there, so they are the next
    /// statement's leading comments, past blank lines too, unless comment lines come right
    /// before it (`pending`, which nu puts in their place).
    carried_comments: Vec<Comment>,
    /// The pipes that no command took before the statement ended.
    carried_pipe: CarriedPipes,
    /// What the last token was (`Item` for a whole statement): an `Eol` after an `Eol` is a
    /// blank line, and a `;` or a comment right after a statement belongs to it.
    last: TokenContents,
    /// Whether a statement was pushed. A comment that is not first on its line trails the
    /// statement pushed last, if there is one; otherwise it waits for the next statement.
    pushed: bool,
    /// The lite-parse error of a command nu drops, which leaves no pipeline to carry it (an
    /// `e>|` with nothing before it), the first: the block's.
    dropped_error: Option<LiteError>,
}

/// The start of a block counts as the start of a line.
impl Default for StatementsState {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            carried_comments: Vec::new(),
            carried_pipe: CarriedPipes::default(),
            last: TokenContents::Eol,
            pushed: false,
            dropped_error: None,
        }
    }
}

/// The statements of a block, pushed to `out` as they are parsed, until `out`
/// says to stop. Comments on lines of their own before a statement become its
/// leading comments.
///
/// A loop over the tokens rather than a combinator: what an `Eol`, `;` or
/// comment means depends on the token before it (`state.last`), and the state
/// must survive a stop so that a later call goes on from it.
fn parse_statements<'a>(
    tokens: &mut Tokens<'_, 'a>,
    state: &mut StatementsState,
    out: &mut dyn StatementSink<'a>,
) {
    let working_set = tokens.working_set;
    let StatementsState {
        pending,
        carried_comments,
        carried_pipe,
        last,
        pushed,
        dropped_error,
    } = state;
    while let Some(token) = tokens.peek_token() {
        match token.contents {
            TokenContents::Eol => {
                // nu's lexer turns an end of line before a `|` that starts the next line into
                // that `|`, and drops the ones before the comment lines right before it: they end
                // no line, so they make no blank line (`# d\n\n| def f`).
                let folded = reaches_a_pipe_past_comment_lines(tokens.all(), tokens.position() + 1);
                if !folded {
                    if *last == TokenContents::Eol {
                        pending.clear();
                    }
                    *last = TokenContents::Eol;
                }
                tokens.next_token();
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
                let comment = Comment { span: token.span };
                match last {
                    // Beside a `;`, or after a comment with its line end folded (above): nu's lite
                    // parser drops the comment lines before it for this one, which it gives the
                    // next command (`# a`, `; # b`, `def f [] {}`).
                    TokenContents::Semicolon | TokenContents::Comment => {
                        pending.clear();
                        carried_comments.push(comment);
                    }
                    TokenContents::Eol => pending.push(comment),
                    _ if *pushed => {
                        if let Some(pipeline) = out.last_mut() {
                            pipeline.trailing_comments.push(comment);
                        }
                    }
                    _ => pending.push(comment),
                }
                tokens.next_token();
                *last = TokenContents::Comment;
            }
            _ => {
                let start = tokens.position();
                let start_span = token.span;
                let before = StatementStart {
                    errors: working_set.error_count(),
                    longest_name: working_set.longest_decl_name(),
                };
                // The state is up to date before each push: `out` may stop the loop there, and a
                // later call goes on from the state.
                let mut lite_end = None;
                let mut lite_error = None;
                let go_on = match parse_pipeline(
                    tokens,
                    pending,
                    carried_comments,
                    carried_pipe,
                    &mut lite_end,
                    &mut lite_error,
                    false,
                ) {
                    Ok(Some(mut pipeline)) => {
                        *pushed = true;
                        *last = TokenContents::Item;
                        // A comment right after the statement, on its line, is the statement's
                        // (for a definition, part of its description), so it goes with it;
                        // after its `;` it is the next statement's, as in nu.
                        if let Some(comment) = tokens
                            .peek_token()
                            .filter(|next| next.contents == TokenContents::Comment)
                        {
                            working_set.add_comment(comment.span);
                            let comment_after = Comment { span: comment.span };
                            if pipeline.terminator.is_some() {
                                carried_comments.push(comment_after);
                            } else {
                                pipeline.trailing_comments.push(comment_after);
                            }
                            tokens.next_token();
                            *last = TokenContents::Comment;
                        }
                        out.push(pipeline, before)
                    }
                    Ok(None) => {
                        // Only pipes: a blank line after them drops the comment lines before them.
                        if tokens
                            .peek_token()
                            .is_none_or(|next| next.contents != TokenContents::Semicolon)
                        {
                            pending.clear();
                        }
                        *last = TokenContents::Item;
                        if let Some(error) = lite_error {
                            dropped_error.get_or_insert(error);
                        }
                        true
                    }
                    Err(error) => {
                        // An error the statement failed on was gone past by nothing, even one
                        // marked recovered where it was made, which only the construct's own
                        // parser goes past: the statement is to be parsed again.
                        let mut diagnostic = into_diagnostic(error);
                        diagnostic.recovered = false;
                        working_set.error(diagnostic);
                        let end = match lite_end {
                            // The lite parse found where the statement ends, as nu's does
                            // (past the comments on its lines, its commands'), and the next
                            // statement is parsed on from there.
                            Some(end) => {
                                tokens.reset_to(end);
                                tokens.all()[end - 1].span
                            }
                            None => {
                                tokens.reset_to(start);
                                skip_to_statement_end(tokens)
                            }
                        };
                        *pushed = true;
                        *last = TokenContents::Item;
                        out.push_garbage(
                            start_span.merge(end),
                            take_leading_comments(pending, carried_comments),
                            lite_error,
                            before,
                        )
                    }
                };
                if !go_on {
                    return;
                }
            }
        }
    }
}

/// The leading comments of the statement that starts now: the comment lines right before it
/// (`pending`), or else the comments carried to it ([`StatementsState::carried_comments`]),
/// as nu's lite parser gives them to a command. Both are taken.
fn take_leading_comments(pending: &mut Vec<Comment>, carried: &mut Vec<Comment>) -> Vec<Comment> {
    if pending.is_empty() {
        std::mem::take(carried)
    } else {
        carried.clear();
        std::mem::take(pending)
    }
}

/// Move the comments of `comments[from..]`, recorded after a `|` (or `e>|`) that ended their
/// pipeline, to `carried` when something comes before them on their line: nu's lite parser puts
/// those in the command it starts after a `|`, the next statement's. The comment lines among
/// them are dropped from the pipeline (they stay in `Ast::comments`): nu clears them at the
/// blank line that ends the pipeline.
fn carry_comments_beside(
    working_set: &WorkingSet<'_>,
    comments: &mut Vec<Comment>,
    from: usize,
    carried: &mut Vec<Comment>,
) {
    for comment in comments.drain(from..) {
        let before = working_set.get_span_contents(Span::new(0, comment.span.start));
        let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
        if !before[line_start..].trim().is_empty() {
            carried.push(comment);
        }
    }
}

/// The pipes a statement leaves to the next one, which nu's lite parser keeps on the command it
/// has not started yet.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CarriedPipes {
    /// The last `|` before the next command ([`PipelineElement::pipe`]).
    before: Option<Span>,
    /// What nu's `command.pipe` holds ([`PipelineElement::lite_pipe`]).
    lite: Option<Span>,
}

/// Whether `tokens` from `at` is a `|`, past comment lines.
fn reaches_a_pipe_past_comment_lines(tokens: &[Token], mut at: usize) -> bool {
    let contents = |index: usize| tokens.get(index).map(|token| token.contents);
    while contents(at) == Some(TokenContents::Comment)
        && contents(at + 1) == Some(TokenContents::Eol)
    {
        at += 2;
    }
    contents(at) == Some(TokenContents::Pipe)
}

/// One pipeline (nu's `parse_pipeline`): commands separated by `|`. `None`
/// when there was no command at all (a lone `|` before a blank line), the `|`
/// then left in `carried_pipe`: nu's lite parser keeps it for the next command,
/// which starts with it as with a leading `|`. A `;` right after a `|` never
/// gets here: the lexer refuses it, as nu's does. The pipeline takes
/// `leading_comments` (or else `carried_comments`) only when it parses; a
/// comment on the line of a `|` that ends it goes to `carried_comments`.
/// `lite_end` and `found_lite_error` tell where the lite parse ended the statement and its
/// first lite-parse error, for a statement that fails after it. With `lite_only` the commands
/// are not parsed: the lite parse is done and the result is `None` ([`first_lite_error`]).
fn parse_pipeline<'a>(
    tokens: &mut Tokens<'_, 'a>,
    leading_comments: &mut Vec<Comment>,
    carried_comments: &mut Vec<Comment>,
    carried_pipe: &mut CarriedPipes,
    lite_end: &mut Option<usize>,
    found_lite_error: &mut Option<LiteError>,
    lite_only: bool,
) -> ParseResult<Option<Pipeline<'a>>> {
    let working_set = tokens.working_set;
    // The comments carried to this pipeline; any after are carried on from it.
    let mut carried_to_it = carried_comments.len();
    // The lite parse first: collect the commands, then parse them, because a
    // command is parsed differently when it is one element of a longer pipeline.
    let mut lite_commands: Vec<(Option<Span>, LiteCommand)> = Vec::new();
    let mut trailing_comments = Vec::new();
    let mut pipe: Option<Span> = carried_pipe.before.take();
    // nu's `command.pipe` while its command is empty. A pipe that ends no command stays there, and
    // the next command keeps it unless a pipe ends that command too.
    let mut lite_pipe = carried_pipe.lite.take();
    // Whether nu has not ended the last command yet, so that a `|` now ends it.
    let mut command_open = false;
    // A `|` after the last command that no command follows.
    let mut dangling_pipe = None;
    // The last command nu dropped, when no command came after it: the `|` before it and its own
    // `e>|`.
    let mut trailing_dropped: Option<(Option<Span>, Option<Span>)> = None;
    // The first lite-parse error of the pipeline's commands.
    let mut lite_error = None;
    // The error of a command dropped before `lite_error` was set, not recorded yet.
    let mut dropped_error = None;
    // Whether the last command goes on to one on the next line with no `|` between
    // (`LiteCommand::joins_next_line`).
    let mut joins_next_line = false;
    // Each turn takes the `|`s before a command, then the command. A `|` on the command's
    // line is left for the next turn; an `e>|` is taken with the command (`pipe_after`).
    'commands: loop {
        // A pipeline may start with `|` (`( | str join)`) and `a | | b` is
        // `a | b`: the empty commands are dropped.
        let mut first_pipe = None;
        let from = trailing_comments.len();
        while let Some(token) = tokens
            .peek_token()
            .filter(|token| token.contents == TokenContents::Pipe)
        {
            pipe = Some(token.span);
            first_pipe.get_or_insert(token.span);
            match lite_commands.last_mut().filter(|_| command_open) {
                Some((_, lite_command)) => lite_command.lite_pipe = Some(token.span),
                None => lite_pipe = Some(token.span),
            }
            command_open = false;
            tokens.next_token();
            let after = after_pipe(tokens, true, &mut trailing_comments)?;
            // After a `|` that no command came before, nu's lite parser reads comments as it does
            // before a statement: one beside it drops the comment lines before it, and comment
            // lines after it take their place. Those are this statement's when a command follows.
            if lite_commands.is_empty() {
                let next = tokens.peek_token().map(|next| next.contents);
                // Before another `|` their line ends are folded, so each comes beside the last.
                let folded = next == Some(TokenContents::Pipe);
                // Only a blank line drops comment lines, and a `;` is none.
                let keeps_lines =
                    matches!(after, AfterPipe::Command) || next == Some(TokenContents::Semicolon);
                for comment in trailing_comments.drain(from..) {
                    let before = working_set.get_span_contents(Span::new(0, comment.span.start));
                    let line_start = before.rfind('\n').map_or(0, |newline| newline + 1);
                    if folded || !before[line_start..].trim().is_empty() {
                        leading_comments.clear();
                        carried_comments.push(comment);
                    } else if keeps_lines {
                        leading_comments.push(comment);
                    }
                }
                if matches!(after, AfterPipe::Command) {
                    carried_to_it = carried_comments.len();
                }
            }
            if let AfterPipe::Dangling = after {
                // nu ends the last command at the first `|` after it; with more, the empty
                // commands between hand the last on to the next statement (`a | |`).
                if !lite_commands.is_empty() {
                    dangling_pipe = first_pipe;
                    if pipe != first_pipe {
                        carried_pipe.before = pipe;
                    }
                }
                // A `;` is no blank line, so the comment lines before it stay (`1 |`, `# a`, `;`).
                if tokens
                    .peek_token()
                    .is_some_and(|next| next.contents == TokenContents::Semicolon)
                {
                    carried_comments.extend(trailing_comments.drain(from..));
                }
                carry_comments_beside(working_set, &mut trailing_comments, from, carried_comments);
                break 'commands;
            }
        }
        if pipe.is_none() && !lite_commands.is_empty() {
            let joins = std::mem::take(&mut joins_next_line);
            if joins && command_on_next_line(tokens) {
                // The end of line the redirection stopped at.
                tokens.next_token();
            } else if joins && pipe_on_a_later_line(tokens.all(), tokens.position() + 1).is_some() {
                // A blank line, then a `|`: nu's lexer makes the blank line's end of line that
                // `|`, which goes on with the pipeline.
                tokens.next_token();
                continue;
            } else {
                // After a command: the pipeline goes on only through a `|` on a later line.
                if !take_pipe_on_later_line(tokens, &mut trailing_comments)? {
                    break;
                }
                continue;
            }
        }
        // nu's lite parser reads attribute lines where an `@` item follows an end of line or a
        // `;`: before a statement's first command, and before a command starting a line.
        let starts_line = tokens
            .position()
            .checked_sub(1)
            .and_then(|previous| tokens.all().get(previous))
            .is_some_and(|previous| {
                matches!(
                    previous.contents,
                    TokenContents::Eol | TokenContents::Semicolon
                )
            });
        let mut lite_command = parse_lite_command(tokens, pipe.is_none() || starts_line)?;
        trailing_comments.extend(lite_command.comments.iter().copied());
        let pipe_after = lite_command.pipe_after;
        lite_error = lite_error.or(lite_command.lite_error.as_deref().copied());
        joins_next_line = lite_command.joins_next_line;
        // An `e>|` with nothing before it ends a command nu drops: the `|` before it, if any,
        // is the pipe before the next command.
        let dropped = lite_command.parts.is_empty()
            && lite_command.attributes.is_empty()
            && lite_command.redirections.is_empty();
        if dropped {
            dropped_error = dropped_error.or(lite_command.lite_error.as_deref().copied());
            trailing_dropped = Some((pipe, pipe_after));
            lite_pipe = pipe_after.or(lite_pipe);
        } else {
            trailing_dropped = None;
            lite_command.lite_pipe = pipe_after.or(lite_pipe.take());
            // An `e>|` ended it, or the end of line after a redirection without a target.
            command_open = pipe_after.is_none() && !lite_command.joins_next_line;
            lite_commands.push((pipe.take(), lite_command));
        }
        if pipe_after.is_some() {
            // The `e>|` is the last pipe before the next command, also after a command nu
            // dropped: nu's lite parser sets it on the command it does not take, which the
            // next command's items then go into (`ls e>| e>| x`, `ls | e>| x`).
            pipe = pipe_after;
            // After an `e>|` nu goes on to a later line only through a `|` that starts it.
            let from = trailing_comments.len();
            if let AfterPipe::Dangling = after_pipe(tokens, false, &mut trailing_comments)? {
                carry_comments_beside(working_set, &mut trailing_comments, from, carried_comments);
                break;
            }
        }
    }
    if lite_commands.is_empty() {
        // An `e>|` alone leaves no pipeline: nu reports its error as the block's and goes on,
        // and so does the consumer, which the block or its statement loop hands it to.
        // Its `e>|` is the pipe of the next command nu reads, as a lone `|` is.
        if let Some(error) = dropped_error {
            working_set.error(lite_diagnostic(error).as_recovered());
            *found_lite_error = Some(error);
            *carried_pipe = CarriedPipes {
                before: pipe,
                lite: lite_pipe,
            };
            return Ok(None);
        }
        // Only pipes (`|` and a blank line): nu drops the empty command.
        *carried_pipe = CarriedPipes {
            before: pipe,
            lite: lite_pipe,
        };
        return Ok(None);
    }
    if let Some(error) = dropped_error {
        working_set.error(lite_diagnostic(error).as_recovered());
    }
    // A pipeline ending with a command nu dropped (`ls |` then `e>|`): the `|` before it stays
    // with the command before, and its `e>|` is the pipe of the next command nu reads.
    if let Some((before, own)) = trailing_dropped {
        if dangling_pipe.is_none() {
            dangling_pipe = before;
        }
        carried_pipe.before = own;
    }
    carried_pipe.lite = lite_pipe;
    // The lite parse is done: the statement ends here, unless a `|` without a command after
    // it hands on to the next one (`a | |`) or ends it before a blank line.
    if dangling_pipe.is_none() && carried_pipe.before.is_none() {
        *lite_end = Some(tokens.position());
    }
    *found_lite_error = lite_error;
    if lite_only {
        return Ok(None);
    }
    let single = lite_commands.len() == 1;
    if !single {
        for (_, lite_command) in &lite_commands {
            check_loop_in_pipeline(working_set, lite_command);
        }
    }
    let mut elements: Vec<PipelineElement<'a>> = Vec::with_capacity(lite_commands.len());
    for (pipe, lite_command) in &lite_commands {
        let redirection_spans = lite_command
            .redirections
            .iter()
            .flat_map(|(operator, target)| {
                std::iter::once(operator.span).chain(target.map(|target| target.span))
            })
            .collect();
        let outer = working_set.replace_element_redirections(redirection_spans);
        let parsed = parse_pipeline_element(working_set, lite_command, !single);
        working_set.replace_element_redirections(outer);
        let (expr, redirection) = parsed?;
        let start = pipe.map_or(expr.span.start, |pipe| pipe.start);
        let end = redirection.as_ref().map_or(expr.span.end, |redirection| {
            redirection.span().end.max(expr.span.end)
        });
        // From the first attribute item, or else the first part, to the last part.
        let first = lite_command
            .attributes
            .first()
            .and_then(|line| line.first())
            .or(lite_command.parts.first());
        let last = lite_command
            .parts
            .last()
            .or_else(|| lite_command.attributes.last().and_then(|line| line.last()));
        let parts = match (first, last) {
            (Some(first), Some(last)) => first.span.merge(last.span),
            _ => expr.span,
        };
        elements.push(PipelineElement {
            span: Span::new(start, end),
            pipe: *pipe,
            lite_pipe: lite_command.lite_pipe,
            expr,
            redirection,
            parts,
        });
    }
    let span = elements[0].span.merge(
        elements
            .last()
            .map_or(elements[0].span, |element| element.span),
    );
    let terminator = match tokens.peek_token() {
        Some(token) if token.contents == TokenContents::Semicolon => {
            tokens.next_token();
            Some(token.span)
        }
        _ => None,
    };
    // nu's lite parser gives the comment lines before a command to its first item that is no
    // attribute, so attribute lines with no command after them take none: those comments stay
    // for the next statement (`# a`, `@x`, `# b`, `def f [] {}` describes `f` with both).
    let attributes_only = matches!(
        lite_commands.as_slice(),
        [(_, lite_command)] if lite_command.parts.is_empty()
            && lite_command.redirections.is_empty()
            && !lite_command.attributes.is_empty()
    );
    let carried_on = carried_comments.split_off(carried_to_it);
    let leading_comments = match attributes_only {
        true => Vec::new(),
        false => take_leading_comments(leading_comments, carried_comments),
    };
    *carried_comments = carried_on;
    Ok(Some(Pipeline {
        span,
        elements,
        leading_comments,
        trailing_comments,
        terminator,
        dangling_pipe,
        lite_error: lite_error.map(Box::new),
        lex_error: None,
    }))
}

/// Whether a command starts the line after the end of line `tokens` is at, for a pipeline that
/// goes on there (`LiteCommand::joins_next_line`): not a blank line, a comment line, a `;` or
/// the end of the block.
fn command_on_next_line(tokens: &Tokens<'_, '_>) -> bool {
    let all = tokens.all();
    let at = tokens.position();
    all.get(at)
        .is_some_and(|token| token.contents == TokenContents::Eol)
        && all.get(at + 1).is_some_and(|next| {
            !matches!(
                next.contents,
                TokenContents::Eol
                    | TokenContents::Comment
                    | TokenContents::Semicolon
                    | TokenContents::Pipe
                    | TokenContents::Eof
            )
        })
}

/// Report a `while` or `loop` that is one command of a longer pipeline (nu's `parse_pipeline`):
/// loops are statements without output, so like `for` (refused in `parse_expression`) they
/// can't be piped. The head is the first item after any `NAME=value` environment shorthands, as
/// there, and an attribute line's first item counts as one. nu reports each such command
/// before it parses the pipeline, which it then parses as any other, and so does the consumer:
/// the error is recorded as recovered.
fn check_loop_in_pipeline(working_set: &WorkingSet<'_>, lite_command: &LiteCommand) {
    let is_env_shorthand = |token: &&Token| {
        let text = working_set.get_span_contents(token.span);
        text.find('=')
            .is_some_and(|equals| is_env_variable_name(&text[..equals]))
    };
    let head = lite_command
        .attributes
        .iter()
        .flatten()
        .chain(&lite_command.parts)
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
    lite_command: &LiteCommand,
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
            let mut items = lite_command.parts.clone();
            for (operator, target) in &lite_command.redirections {
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
                .flatten()
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
        for (operator, target) in &lite_command.redirections {
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
