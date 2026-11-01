//! The parser proper, laid out like nu-parser: each file holds the functions
//! of its nu-parser namesake, under the same names (`parse_block`,
//! `parse_pipeline_element`, `parse_expression`, `parse_value`, `parse_call`,
//! `parse_def`, ...).
//!
//! | File | What it parses | nu-parser |
//! | --- | --- | --- |
//! | [`lite_parser`] | groups a block's tokens into commands | `lite_parser.rs` |
//! | [`parse_pipelines`] | blocks, pipelines, pipeline elements, redirections | `parse_pipelines.rs` |
//! | [`parse_expressions`] | expressions, builtin dispatch, math, values, lists, records, blocks, closures, match blocks | `parse_expressions.rs` |
//! | [`parse_calls`] | calls and their arguments, external and `%` calls, attributes, keyword-command signatures | `parse_calls.rs` |
//! | [`parse_keywords`] | parser keywords, and [`parse_keywords::KeywordCall`]: nu's flags of a keyword command | `parse_keywords.rs` |
//! | [`parse_def`] | `def`, `extern`, `for`, predeclaration | `parse_def.rs` |
//! | [`parse_bindings`] | `let`, `mut`, `const` | `parse_bindings.rs` |
//! | [`parse_alias`] | `alias` | `parse_alias.rs` |
//! | [`parse_type_alias`] | `type` | `parse_type_alias.rs` |
//! | [`parse_module`] | `module`, `use`, `export`, `export-env` | `parse_module.rs` |
//! | [`parse_source`] | `where` | `parse_source.rs` |
//! | [`parse_control_flow`] | `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue` | (ordinary commands in nu) |
//! | [`parse_signatures`] | signatures, variable declarations, input/output types | `parse_signatures.rs` |
//! | [`parse_shape_specs`] | type annotations and completers | `parse_shape_specs.rs` |
//! | [`parse_patterns`] | `match` patterns | `parse_patterns.rs` |
//! | [`parse_literals`] | numbers, units, datetimes, binary, strings, variables, cell paths, ranges | `parse_literals.rs` |
//! | [`parse_helpers`] | small shared helpers | `parse_helpers.rs` |
//!
//! Every function takes the [`WorkingSet`] (nu's `StateWorkingSet`): the
//! source, the known command names, and what the parse collects on the side.
//! A function that reads one item takes it as a span, like nu's
//! `parse_value(working_set, span, shape)`; one that reads a sequence of items
//! takes a [`tokens::Tokens`] stream, which carries the working set and which
//! winnow's combinators drive (`repeat`, `opt`, `alt`, `separated`,
//! `expression`, ...). Nested constructs (`[...]`, `{...}`, `(...)`) are lexed
//! again from their interior, as in nu.

pub(crate) mod lite_parser;
pub(crate) mod parse_alias;
pub(crate) mod parse_bindings;
pub(crate) mod parse_calls;
pub(crate) mod parse_control_flow;
pub(crate) mod parse_def;
pub(crate) mod parse_expressions;
pub(crate) mod parse_helpers;
pub(crate) mod parse_keywords;
pub(crate) mod parse_literals;
pub(crate) mod parse_module;
pub(crate) mod parse_patterns;
pub(crate) mod parse_pipelines;
pub(crate) mod parse_shape_specs;
pub(crate) mod parse_signatures;
pub(crate) mod parse_source;
pub(crate) mod parse_type_alias;
pub(crate) mod tokens;
pub(crate) mod working_set;

use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

use crate::winnow::syntax::ast::{Ast, Block, Expr, Expression};
use crate::winnow::syntax::error::Diagnostic;
use crate::winnow::syntax::lex::{LexOptions, TokenContents};
use crate::winnow::syntax::span::Span;

pub use parse_def::{Definitions, PredeclaredDef, TypeAliasPredecl};
pub use parse_pipelines::{BlockSink, BlockStatements};
use working_set::Collected;
pub(crate) use working_set::WorkingSet;
pub use working_set::{AliasedKeyword, CommandLookup, DeclKind};

/// Configuration for a parse.
///
/// The only knowledge the parser needs beyond the grammar is the set of
/// commands that exist, as nu-parser has them in its engine state:
///
/// * multi-word names, so that `str trim --left` is a call to `str trim`
///   rather than a call to `str` with a `trim` argument;
/// * which heads are commands at all: a head that is neither configured nor
///   defined in the file is an external command, whose arguments are
///   external strings (`git log 0b2d1f4..HEAD`), as in nu;
/// * which commands are built-in ([`CommandType::Builtin`]), the only ones
///   `%name` may call;
/// * which commands take a row condition (`any`, `take while`), whose
///   arguments parse as one condition, as `where`'s.
///
/// Commands defined in the file being parsed (`def "my cmd" ...`) are always
/// recognised. With no commands configured ([`ParseConfig::empty`]) every head
/// is one word and a call, and the rules that need to know whether a command
/// exists are not applied.
#[derive(Clone, Debug, Default)]
pub struct ParseConfig {
    commands: Arc<ConfiguredCommands>,
}

/// The commands of a [`ParseConfig`], behind one `Arc` so that cloning the
/// configuration for each parse stays cheap.
#[derive(Clone, Debug, Default)]
struct ConfiguredCommands {
    /// Every configured command name.
    all: CommandSet,
    /// The names in `all` that are built-in commands.
    builtins: NameSet,
    /// The names whose first positional parameter is a row condition.
    row_conditions: NameSet,
}

/// The kind of a configured command (nu-protocol's `CommandType`, for the
/// kinds a fresh `nu` has at the top level).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandType {
    /// A command implemented in Rust (`ls`, `str trim`); `%name` may call it.
    Builtin,
    /// A parser keyword (`def`, `if`, `let`).
    Keyword,
    /// A command written in Nushell (the standard library's prelude, a
    /// `use`d module's commands).
    Custom,
}

/// Known command names plus an index of the first words of multi-word names,
/// so that the common single-word head needs no string building at all.
#[derive(Clone, Debug, Default)]
pub(crate) struct CommandSet {
    /// Every name, its words separated by single spaces.
    names: NameSet,
    /// The first word of each multi-word name.
    prefixes: NameSet,
    /// The length of the longest name.
    longest: usize,
}

/// A set of command names, hashed with [`NameHasher`].
type NameSet = HashSet<Box<str>, BuildHasherDefault<NameHasher>>;

/// A fast hash for short command names: the multiply-rotate hash rustc uses
/// (FxHash). Names are looked up for every call head, and SipHash, the
/// standard library's default, costs more than the rest of the lookup.
#[derive(Default)]
struct NameHasher(u64);

impl NameHasher {
    /// FxHash's multiplier.
    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    /// Mix one word into the hash.
    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(Self::SEED);
    }
}

impl Hasher for NameHasher {
    /// Eight bytes per step, then the tail byte by byte.
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.add(u64::from_le_bytes(*word));
        }
        for &byte in rest {
            self.add(u64::from(byte));
        }
    }

    #[inline]
    fn write_u8(&mut self, byte: u8) {
        self.add(u64::from(byte));
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

impl CommandSet {
    /// Add `name`, indexing its first word when it has more than one.
    fn insert(&mut self, name: &str) {
        if let Some((first, _)) = name.split_once(' ') {
            self.prefixes.insert(Box::from(first));
        }
        self.longest = self.longest.max(name.len());
        self.names.insert(Box::from(name));
    }
}

impl ParseConfig {
    /// A configuration that knows the commands of a fresh `nu`.
    pub fn new() -> Self {
        Self::default()
            .add_typed_commands(
                crate::winnow::syntax::builtin_commands::BUILTIN_COMMANDS
                    .iter()
                    .copied(),
            )
            .add_row_condition_commands(
                crate::winnow::syntax::builtin_commands::ROW_CONDITION_COMMANDS
                    .iter()
                    .copied(),
            )
    }

    /// A configuration that knows no commands: every call head is a single word.
    pub fn empty() -> Self {
        Self::default()
    }

    /// A configuration knowing exactly the given command names, each a
    /// built-in command.
    pub fn with_commands<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::default()
            .add_typed_commands(names.into_iter().map(|name| (name, CommandType::Builtin)))
    }

    /// Add custom command names (e.g. from `use`d modules) to the known set.
    pub fn add_commands<I, S>(self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.add_typed_commands(names.into_iter().map(|name| (name, CommandType::Custom)))
    }

    /// Add command names with their kind to the known set.
    pub fn add_typed_commands<I, S>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = (S, CommandType)>,
        S: AsRef<str>,
    {
        let configured = Arc::make_mut(&mut self.commands);
        for (name, command_type) in commands {
            let name = name.as_ref();
            configured.all.insert(name);
            if command_type == CommandType::Builtin {
                configured.builtins.insert(Box::from(name));
            }
        }
        self
    }

    /// Mark commands whose first positional parameter is a row condition
    /// (`any`, `take while`): their arguments parse as one condition, as
    /// `where`'s. Only commands that are known count.
    pub fn add_row_condition_commands<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let configured = Arc::make_mut(&mut self.commands);
        configured
            .row_conditions
            .extend(names.into_iter().map(|name| Box::from(name.as_ref())));
        self
    }

    /// Whether `name` (with words separated by single spaces) is a known command.
    #[inline]
    pub fn is_known(&self, name: &str) -> bool {
        self.commands.all.names.contains(name)
    }

    /// How a call to `name` parses, if it is a known command.
    #[inline]
    pub(crate) fn decl_kind(&self, name: &str) -> Option<DeclKind> {
        self.is_known(name)
            .then(|| match self.commands.row_conditions.contains(name) {
                true => DeclKind::RowCondition,
                false => DeclKind::Builtin,
            })
    }

    /// Whether `name` is a known built-in command, which `%name` may call.
    #[inline]
    pub fn is_builtin(&self, name: &str) -> bool {
        self.commands.builtins.contains(name)
    }

    /// Whether `word` is the first word of some known multi-word command.
    #[inline]
    pub fn is_prefix(&self, word: &str) -> bool {
        self.commands.all.prefixes.contains(word)
    }

    /// The length of the longest known command name.
    #[inline]
    pub fn longest_name(&self) -> usize {
        self.commands.all.longest
    }

    /// Number of known command names.
    pub fn len(&self) -> usize {
        self.commands.all.names.len()
    }

    /// `true` if no commands are known.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.commands.all.names.is_empty()
    }
}

/// Parse `source` into an AST plus any diagnostics (nu's `parse`).
pub(crate) fn parse<'a>(source: &'a str, config: &ParseConfig) -> (Ast<'a>, Vec<Diagnostic>) {
    let working_set = WorkingSet::new(source, config);
    let whole_source = Span::new(0, source.len());
    let shebang = source.starts_with("#!").then(|| {
        let end = source.find('\n').unwrap_or(source.len());
        Span::new(0, end)
    });
    let block = match working_set.lex_block(whole_source, LexOptions::BLOCK) {
        Ok(tokens) => parse_pipelines::parse_block(
            tokens::Tokens::from_lexed(&working_set, &tokens),
            whole_source,
        ),
        Err(diagnostic) => {
            working_set.error(diagnostic);
            Block {
                span: whole_source,
                pipelines: Vec::new(),
            }
        }
    };
    let Collected {
        comments,
        ignored,
        parse_errors,
    } = working_set.into_collected();
    (
        Ast {
            source,
            block,
            comments,
            shebang,
            ignored,
        },
        parse_errors,
    )
}

/// Parse the block covering `span` of `source` statement by statement (see
/// [`crate::winnow::syntax::parse_block_streaming`]).
pub(crate) fn parse_block_streaming<'a>(
    source: &'a str,
    span: Span,
    lookup: &'a dyn CommandLookup,
    sink: &mut dyn BlockSink<'a>,
) -> Result<(), Vec<Diagnostic>> {
    let (mut statements, definitions) = BlockStatements::new(source, span, lookup, false)?;
    for type_alias in definitions.type_aliases() {
        sink.predecl_type_alias(*type_alias);
    }
    for def in definitions.parse(source, span, lookup) {
        sink.predecl(def);
    }
    statements.parse(source, lookup, &mut |pipeline, diagnostics, _| {
        sink.statement(pipeline, diagnostics)
    });
    Ok(())
}

/// What [`parse_items`] reads text as: the values of nu's `parse_multispan_value` that take
/// every item it gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ItemsValue {
    /// A math expression (`parse_math_expression`), the condition of `if`.
    Math,
    /// A row condition (`parse_row_condition`), the predicate of `take while`.
    RowCondition,
    /// An alias's target read as the argument of `alias` (`parse_expression`), where every
    /// item is a word, pipes and redirections too.
    AliasTarget,
}

/// The items covering `span` of `source`, which a call's arguments read one at a time (`take
/// while --include 1 $it < 3`, a call for help, an alias's target) or an alias read as its
/// body's call, parsed again as the one `value` that nu-parser reads them as there, with the
/// commands `lookup` knows. `None` when that is
/// a syntax error or ends before `span` does.
pub(crate) fn parse_items<'a>(
    source: &'a str,
    span: Span,
    lookup: impl CommandLookup + 'a,
    value: ItemsValue,
) -> Option<Expression<'a>> {
    let working_set = WorkingSet::with_lookup(source, span, lookup);
    let mut tokens = working_set.lex(span, LexOptions::BLOCK).ok()?;
    for token in &mut tokens {
        let is_item = match value {
            // As `parse_alias` hands an alias's target to the call parser.
            ItemsValue::AliasTarget => token.contents != TokenContents::Eof,
            // The items were a call's, which nu reads as words: a redirection operator among
            // them (a lone `let` takes its redirections as its own items) is one.
            ItemsValue::Math | ItemsValue::RowCondition => {
                matches!(token.contents, TokenContents::Redirection(_))
            }
        };
        if is_item {
            token.contents = TokenContents::Item;
        }
    }
    let items = tokens
        .iter()
        .filter(|token| token.contents == TokenContents::Item)
        .count();
    let tokens = tokens::Tokens::from_lexed(&working_set, &tokens);
    let parsed = match value {
        ItemsValue::Math => parse_expressions::parse_math_expression(tokens),
        ItemsValue::RowCondition => parse_source::parse_condition(tokens),
        ItemsValue::AliasTarget => {
            parse_expressions::parse_expression(tokens, parse_expressions::Position::Element)
        }
    };
    // A diagnostic the parser went on past leaves a garbage node, which the consumer reads
    // again with nu's parser, reporting nu's error (an item where an operator goes, `> 0`; a
    // shorthand without a command, `FOO=1`).
    let recovered = working_set
        .take_errors_from(0)
        .iter()
        .all(|diagnostic| diagnostic.recovered);
    let parsed = parsed
        .ok()
        .filter(|expression| expression.span == span && recovered);
    // Math items the parser cannot read to their end (`true 1 +`, the condition of `if true 1 +
    // 1`) are a garbage leaf, which the consumer reads again with nu's math parser, as nu reads
    // those very items (`parse_multispan_value`), reporting its errors with what it recovers.
    match parsed {
        None if value == ItemsValue::Math && items > 1 => {
            Some(Expression::new(Expr::Garbage, span))
        }
        parsed => parsed,
    }
}

/// The call covering `span` of `source` whose head is its first `head_words` items
/// ([`parse_calls::parse_call_with_head_words`]), with the commands `lookup` knows. `None` when
/// it does not parse to the end of `span`; an error the parser went on past leaves a garbage
/// node, which the consumer reads with nu's error.
pub(crate) fn parse_headed_call<'a>(
    source: &'a str,
    span: Span,
    lookup: impl CommandLookup + 'a,
    head_words: usize,
) -> Option<Expression<'a>> {
    let working_set = WorkingSet::with_lookup(source, span, lookup);
    let tokens = working_set.lex(span, LexOptions::BLOCK).ok()?;
    let tokens = tokens::Tokens::from_lexed(&working_set, &tokens);
    let parsed = parse_calls::parse_call_with_head_words(tokens, head_words);
    let recovered = working_set
        .take_errors_from(0)
        .iter()
        .all(|diagnostic| diagnostic.recovered);
    parsed
        .ok()
        .filter(|expression| expression.span == span && recovered)
}

/// The arms of the `{ ... }` covering `span` of `source`, which a call's arguments read as a
/// block (`match 1 { _ => 1 } extra`, a `match` whose extra argument makes it an ordinary call),
/// parsed again as the match block nu's value parser reads there
/// (`parse_match_block_expression`), with the commands `lookup` knows. `None` when the arms
/// have an error, which the consumer leaves to nu's value parser, as for a match block the
/// `match` statement could not read ([`parse_control_flow`]'s `garbage_match_block`).
pub(crate) fn parse_match_arms<'a>(
    source: &'a str,
    span: Span,
    lookup: impl CommandLookup + 'a,
) -> Option<Vec<crate::winnow::syntax::ast::MatchArm<'a>>> {
    let working_set = WorkingSet::with_lookup(source, span, lookup);
    let arms = parse_expressions::parse_match_block_expression(&working_set, span).ok()?;
    (working_set.error_count() == 0).then_some(arms)
}
