//! The state every parser function shares during one parse.
//!
//! [`WorkingSet`] plays the part of nu-parser's `StateWorkingSet`, and its
//! methods carry the same names where the two overlap: `get_span_contents`,
//! `error`, `enter_scope`/`exit_scope`, `find_decl` and `add_predecl`. Parser
//! functions take it as their first argument, `working_set: &WorkingSet<'a>`,
//! exactly as nu-parser's do. Everything it collects on the side (comments,
//! ignored text, diagnostics, declared command names) sits behind a
//! [`RefCell`], so the combinators that capture it can share one `&WorkingSet`.

use std::cell::{Cell, RefCell};

use crate::ast::Comment;
use crate::error::Diagnostic;
use crate::input::{Input, ParseResult};
use crate::lex::{self, GroupEnds, LexOptions, Token, TokenContents};
use crate::span::Span;

use super::{CommandSet, ParseConfig};

/// Where a command name known to the parser comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    /// Declared with `def`, `extern` or `alias` in an enclosing block.
    Declared,
    /// A `def --wrapped` whose rest parameter has no type (`def --wrapped f
    /// [...rest] {}`), or an alias of one: nu gives that parameter the
    /// `external_arg` shape, so its arguments parse as an external command's.
    Wrapped,
    /// An `alias` whose target is an external command (`alias g = git`): nu
    /// parses a call to it as an external call, with external arguments.
    ExternalAlias,
    /// A command whose first positional parameter is a row condition (`any`,
    /// `take while`, `record where`, ...), or an alias of one that gives it no
    /// arguments: nu reads its arguments, from the first one to the end, as one
    /// condition, as `where`'s (when the first one is a flag, whose value nu
    /// takes by the flag's shape, the parser reads them one by one).
    RowCondition,
    /// An alias of `if`, `match` or `try` that gives it no arguments (`alias m =
    /// match`): a call to it parses as that keyword's statement.
    KeywordAlias(AliasedKeyword),
    /// One of the configured (built-in) commands of [`ParseConfig`].
    Builtin,
}

/// A keyword with a statement of its own that nu lets an alias name: `if`,
/// `match` and `try` (`alias m = match`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasedKeyword {
    /// `if`.
    If,
    /// `match`.
    Match,
    /// `try`.
    Try,
}

impl AliasedKeyword {
    /// The keyword named `name`, if it is one of them.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "if" => Some(Self::If),
            "match" => Some(Self::Match),
            "try" => Some(Self::Try),
            _ => None,
        }
    }
}

/// The commands declared in one block, closure or module body.
#[derive(Debug, Default)]
struct Scope {
    /// The names declared here.
    commands: CommandSet,
    /// The names in `commands` whose calls parse differently: wrapped commands
    /// and aliases of external commands, of row-condition commands and of
    /// keywords. Few, so a list.
    kinds: Vec<(Box<str>, DeclKind)>,
}

impl Scope {
    /// Declare `name`, and record how calls to it parse.
    fn declare(&mut self, name: &str, kind: DeclKind) {
        self.commands.insert(name);
        self.kinds.retain(|(declared, _)| **declared != *name);
        if kind != DeclKind::Declared {
            self.kinds.push((Box::from(name), kind));
        }
    }

    /// How calls to `name`, declared in this scope, parse.
    fn kind(&self, name: &str) -> DeclKind {
        self.kinds.iter().find(|(declared, _)| **declared == *name).map_or(DeclKind::Declared, |&(_, kind)| kind)
    }
}

/// The commands that exist, asked of an engine instead of a [`ParseConfig`].
///
/// nu-parser implements this over its `StateWorkingSet` to parse with the
/// engine's live declarations: the statements of a block are then parsed one
/// at a time ([`crate::parse_block_streaming`]), so a command that a `use`,
/// `alias` or `overlay use` brings into scope resolves in the statements after
/// it, exactly as in nu. Names declared in the text being parsed (`def`,
/// `extern`, `alias`) are still tracked by the parser's own scopes and shadow
/// these.
pub trait CommandLookup {
    /// Whether `name` (words separated by single spaces) is a command, and how
    /// calls to it parse.
    fn find_decl(&self, name: &str) -> Option<DeclKind>;
    /// Whether `word` is the first word of some multi-word command.
    fn is_decl_name_prefix(&self, word: &str) -> bool;
    /// An upper bound on the length of every command name, so the parser need not look up a
    /// longer one. Without one (the default), every candidate is looked up.
    fn longest_name(&self) -> usize {
        usize::MAX
    }
    /// Whether some declaration named `name` is a built-in command, visible or
    /// not (shadowed or hidden): what `%name` may call, as nu-parser's
    /// `find_decl_with_command_type(.., CommandType::Builtin)` answers it.
    fn is_builtin_decl(&self, name: &str) -> bool;
    /// Whether to leave unparsed, as an empty block, a nested block with a
    /// statement that changes which commands exist (`use`, `module`, `export`,
    /// `overlay`, `hide`, `source`, `run`), an inline module's body among them.
    /// A front end that parses such a block again, with the commands its
    /// statements bring in, answers `true` and saves parsing it twice. `false`
    /// (the default) parses every block.
    fn defers_command_changing_blocks(&self) -> bool {
        false
    }
}

impl<T: CommandLookup + ?Sized> CommandLookup for &T {
    fn find_decl(&self, name: &str) -> Option<DeclKind> {
        (**self).find_decl(name)
    }

    fn is_decl_name_prefix(&self, word: &str) -> bool {
        (**self).is_decl_name_prefix(word)
    }

    fn longest_name(&self) -> usize {
        (**self).longest_name()
    }

    fn is_builtin_decl(&self, name: &str) -> bool {
        (**self).is_builtin_decl(name)
    }

    fn defers_command_changing_blocks(&self) -> bool {
        (**self).defers_command_changing_blocks()
    }
}

/// The source text, the known command names, and what one parse collects.
pub struct WorkingSet<'a> {
    /// The complete source text; every span indexes into it.
    pub source: &'a str,
    /// The configured commands, consulted when there is no `lookup`.
    config: ParseConfig,
    /// The engine to ask about commands, instead of `config`. Owned, so that a parse on
    /// another thread can own its lookup while the trees it builds borrow only the source.
    lookup: Option<Box<dyn CommandLookup + 'a>>,
    /// Every comment found so far, in no particular order ([`WorkingSet::into_collected`] sorts).
    comments: RefCell<Vec<Comment>>,
    /// Source text nu-parser accepts and discards (see [`crate::ast::Ast::ignored`]).
    ignored: RefCell<Vec<Span>>,
    /// The diagnostics recorded with [`WorkingSet::error`].
    parse_errors: RefCell<Vec<Diagnostic>>,
    /// Command names declared with `def`/`extern`/`alias` in enclosing blocks,
    /// innermost scope last.
    scopes: RefCell<Vec<Scope>>,
    /// Where the bracket groups of the text being parsed close, so that lexing the inside of
    /// a group again does not scan its nested groups again.
    groups: RefCell<GroupEnds>,
    /// The length of the longest name declared in the text so far.
    longest_declared: Cell<usize>,
}

impl std::fmt::Debug for WorkingSet<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkingSet")
            .field("source", &self.source)
            .field("config", &self.config)
            .field("lookup", &self.lookup.is_some())
            .field("parse_errors", &self.parse_errors)
            .finish_non_exhaustive()
    }
}

/// What a parse knows between two statements of a block besides the tokens: the commands
/// its scopes declare and the bracket groups it measured. A block's statements can be parsed
/// a few at a time, each time in a new [`WorkingSet`] (see [`super::BlockStatements`]).
#[derive(Debug, Default)]
pub(crate) struct ParseState {
    /// The declaration scopes, innermost last.
    scopes: Vec<Scope>,
    /// The bracket groups measured so far.
    groups: GroupEnds,
    /// The length of the longest name declared so far.
    longest_declared: usize,
}

/// What a finished parse collected besides the tree.
pub struct Collected {
    /// Every comment, sorted and without duplicates.
    pub comments: Vec<Comment>,
    /// The text nu-parser discards, sorted and without duplicates.
    pub ignored: Vec<Span>,
    /// Every diagnostic, sorted by position.
    pub parse_errors: Vec<Diagnostic>,
}

impl<'a> WorkingSet<'a> {
    /// A working set for parsing `source` with the command names of `config`.
    pub fn new(source: &'a str, config: &ParseConfig) -> Self {
        Self {
            source,
            config: config.clone(),
            lookup: None,
            comments: RefCell::new(Vec::new()),
            ignored: RefCell::new(Vec::new()),
            parse_errors: RefCell::new(Vec::new()),
            scopes: RefCell::new(vec![Scope::default()]),
            groups: RefCell::new(GroupEnds::new(Span::new(0, source.len()))),
            longest_declared: Cell::new(0),
        }
    }

    /// A working set for parsing the text of `span` in `source` with the commands `lookup`
    /// knows. Its [`GroupEnds`] table covers `span` only.
    pub fn with_lookup(source: &'a str, span: Span, lookup: impl CommandLookup + 'a) -> Self {
        let mut working_set = Self::new(source, &ParseConfig::empty());
        working_set.lookup = Some(Box::new(lookup));
        working_set.groups = RefCell::new(GroupEnds::new(span));
        working_set
    }

    /// A working set that goes on from `state`, what an earlier one over the same text knew
    /// ([`WorkingSet::into_state`]), with the commands `lookup` knows.
    pub(crate) fn with_state(source: &'a str, lookup: impl CommandLookup + 'a, state: ParseState) -> Self {
        let mut working_set = Self::new(source, &ParseConfig::empty());
        working_set.lookup = Some(Box::new(lookup));
        working_set.scopes = RefCell::new(state.scopes);
        working_set.groups = RefCell::new(state.groups);
        working_set.longest_declared = Cell::new(state.longest_declared);
        working_set
    }

    /// What this working set knows for a later one to go on from ([`WorkingSet::with_state`]).
    pub(crate) fn into_state(self) -> ParseState {
        ParseState {
            scopes: self.scopes.into_inner(),
            groups: self.groups.into_inner(),
            longest_declared: self.longest_declared.get(),
        }
    }

    /// Lex the text of `span` (nu's `lex` over the span's contents).
    pub fn lex(&self, span: Span, options: LexOptions) -> Result<Vec<Token>, Diagnostic> {
        self.lex_n_tokens(span, options, usize::MAX)
    }

    /// Lex at most `max_tokens` tokens of the text of `span` (nu's `lex_n_tokens`).
    pub fn lex_n_tokens(&self, span: Span, options: LexOptions, max_tokens: usize) -> Result<Vec<Token>, Diagnostic> {
        let mut groups = self.groups.borrow_mut();
        lex::lex_n_tokens_with(self.get_span_contents(span), span.start, options, max_tokens, &mut groups)
    }

    /// The offset in the text of `span` of the bracket closing the group it starts with
    /// ([`lex::group_end`]).
    pub fn group_end(&self, span: Span) -> Option<usize> {
        lex::group_end_with(self.get_span_contents(span), span.start, &mut self.groups.borrow_mut())
    }

    /// The next token of `input`, a stream over part of the source ([`lex::next_token`]).
    pub fn next_token(&self, input: &mut Input<'_>, options: LexOptions) -> ParseResult<Option<Token>> {
        lex::next_token(input, options, &mut self.groups.borrow_mut())
    }

    /// The number of diagnostics recorded so far.
    pub fn error_count(&self) -> usize {
        self.parse_errors.borrow().len()
    }

    /// The diagnostics recorded after the first `count`, removed.
    pub fn take_errors_from(&self, count: usize) -> Vec<Diagnostic> {
        let mut parse_errors = self.parse_errors.borrow_mut();
        let count = count.min(parse_errors.len());
        parse_errors.split_off(count)
    }

    /// The source text of `span`.
    #[inline]
    pub fn get_span_contents(&self, span: Span) -> &'a str {
        span.slice(self.source)
    }

    /// Record a diagnostic (a recovered error: parsing goes on).
    pub fn error(&self, diagnostic: Diagnostic) {
        self.parse_errors.borrow_mut().push(diagnostic);
    }

    /// Record a comment.
    pub fn add_comment(&self, span: Span) {
        self.comments.borrow_mut().push(Comment { span });
    }

    /// Record every comment token in `tokens`.
    pub fn add_comments(&self, tokens: &[Token]) {
        let comments = tokens.iter().filter(|token| token.contents == TokenContents::Comment);
        self.comments.borrow_mut().extend(comments.map(|token| Comment { span: token.span }));
    }

    /// Record text that nu-parser accepts and discards (see [`crate::ast::Ast::ignored`]).
    pub fn add_ignored(&self, span: Span) {
        if !span.is_empty() {
            self.ignored.borrow_mut().push(span);
        }
    }

    /// Drop the ignored text recorded from `offset` on: a statement that turns
    /// out to be a help call is parsed again as an ordinary call.
    pub fn remove_ignored_from(&self, offset: usize) {
        self.ignored.borrow_mut().retain(|span| span.start < offset);
    }

    /// Where `name` is known from, if it is a command: a declaration in an
    /// enclosing block shadows a built-in command of the same name, and the
    /// innermost declaration wins.
    #[inline]
    pub fn find_decl(&self, name: &str) -> Option<DeclKind> {
        let scopes = self.scopes.borrow();
        if let Some(scope) = scopes.iter().rev().find(|scope| scope.commands.names.contains(name)) {
            return Some(scope.kind(name));
        }
        match &self.lookup {
            Some(lookup) => lookup.find_decl(name),
            None => self.config.decl_kind(name),
        }
    }

    /// Whether `name` was declared with `def`, `extern` or `alias` in an
    /// enclosing block (and so shadows a built-in command of that name).
    #[inline]
    pub fn is_declared(&self, name: &str) -> bool {
        self.scopes.borrow().iter().any(|scope| scope.commands.names.contains(name))
    }

    /// Whether `word` is the first word of some known multi-word command.
    #[inline]
    pub fn is_decl_name_prefix(&self, word: &str) -> bool {
        let known = match &self.lookup {
            Some(lookup) => lookup.is_decl_name_prefix(word),
            None => self.config.is_prefix(word),
        };
        known || self.scopes.borrow().iter().any(|scope| scope.commands.prefixes.contains(word))
    }

    /// Whether the lookup has nested blocks that change which commands exist left unparsed
    /// ([`CommandLookup::defers_command_changing_blocks`]).
    #[inline]
    pub fn defers_command_changing_blocks(&self) -> bool {
        self.lookup.as_ref().is_some_and(|lookup| lookup.defers_command_changing_blocks())
    }

    /// An upper bound on the length of every command name: no longer name is known.
    pub fn longest_decl_name(&self) -> usize {
        let known = match &self.lookup {
            Some(lookup) => lookup.longest_name(),
            None => self.config.longest_name(),
        };
        known.max(self.longest_declared.get())
    }

    /// Whether an engine answers the questions about commands ([`CommandLookup`]).
    pub fn has_lookup(&self) -> bool {
        self.lookup.is_some()
    }

    /// Whether the parse knows which commands exist: an engine answers
    /// ([`CommandLookup`]) or the configuration lists some. Rules that need to
    /// know whether a command exists apply only then.
    pub fn has_builtin_decls(&self) -> bool {
        self.lookup.is_some() || !self.config.is_empty()
    }

    /// Whether `name` is a built-in command, even one that a declaration
    /// shadows (what `%name` may call): the engine's answer
    /// ([`CommandLookup::is_builtin_decl`]), or one of the configured ones.
    pub fn is_builtin_decl(&self, name: &str) -> bool {
        match &self.lookup {
            Some(lookup) => lookup.is_builtin_decl(name),
            None => self.config.is_builtin(name),
        }
    }

    /// Declare a command name in the innermost scope before its block is
    /// parsed, so calls to it resolve (nu's `add_predecl`); `kind` is
    /// [`DeclKind::Wrapped`] or [`DeclKind::Declared`].
    pub fn add_predecl(&self, name: &str, kind: DeclKind) {
        self.longest_declared.set(self.longest_declared.get().max(name.len()));
        if let Some(scope) = self.scopes.borrow_mut().last_mut() {
            scope.declare(name, kind);
        }
    }

    /// Declare an alias in the innermost scope once its statement is parsed
    /// (nu adds an alias then, and never predeclares one). `kind` says how a
    /// call to it parses: [`DeclKind::ExternalAlias`] when its target is an
    /// external call, [`DeclKind::Wrapped`] when it is a call to a wrapped
    /// command, [`DeclKind::RowCondition`] or [`DeclKind::KeywordAlias`] when it
    /// is one with no arguments to a row-condition command or to `if`, `match`
    /// or `try`, else [`DeclKind::Declared`].
    pub fn add_alias(&self, name: &str, kind: DeclKind) {
        self.add_predecl(name, kind)
    }

    /// Enter a declaration scope (a block, closure or module body).
    #[inline]
    pub fn enter_scope(&self) {
        self.scopes.borrow_mut().push(Scope::default());
    }

    /// Leave the innermost declaration scope.
    #[inline]
    pub fn exit_scope(&self) {
        self.scopes.borrow_mut().pop();
    }

    /// The comments, ignored text and diagnostics, each sorted by position.
    /// Comments and ignored spans are de-duplicated; diagnostics are kept as
    /// reported.
    pub fn into_collected(self) -> Collected {
        let mut comments = self.comments.into_inner();
        comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
        comments.dedup();
        let mut ignored = self.ignored.into_inner();
        ignored.sort_by_key(|span| (span.start, span.end));
        ignored.dedup();
        let mut parse_errors = self.parse_errors.into_inner();
        parse_errors.sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
        Collected { comments, ignored, parse_errors }
    }
}
