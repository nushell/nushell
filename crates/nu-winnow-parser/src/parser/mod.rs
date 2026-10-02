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
pub(crate) mod tokens;
pub(crate) mod working_set;

use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

use crate::ast::{Ast, Block};
use crate::error::Diagnostic;
use crate::lex::{LexOptions, lex};
use crate::span::Span;

pub use parse_def::PredeclaredDef;
pub use parse_pipelines::BlockSink;
use working_set::Collected;
pub(crate) use working_set::WorkingSet;
pub use working_set::{CommandLookup, DeclKind};

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
///   `%name` may call.
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
#[derive(Debug, Default)]
pub(crate) struct CommandSet {
    names: NameSet,
    prefixes: NameSet,
}

/// A set of command names, hashed with [`NameHasher`].
type NameSet = HashSet<Box<str>, BuildHasherDefault<NameHasher>>;

/// A fast hash for short command names: the multiply-rotate hash rustc uses
/// (FxHash). Names are looked up for every call head, and SipHash, the
/// standard library's default, costs more than the rest of the lookup.
#[derive(Default)]
struct NameHasher(u64);

impl NameHasher {
    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(Self::SEED);
    }
}

impl Hasher for NameHasher {
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
    fn insert(&mut self, name: &str) {
        if let Some((first, _)) = name.split_once(' ') {
            self.prefixes.insert(Box::from(first));
        }
        self.names.insert(Box::from(name));
    }
}

impl ParseConfig {
    /// A configuration that knows the commands of a fresh `nu` (when the
    /// `builtin-commands` feature is enabled, which it is by default).
    pub fn new() -> Self {
        #[cfg(feature = "builtin-commands")]
        {
            Self::default().add_typed_commands(crate::builtin_commands::BUILTIN_COMMANDS.iter().copied())
        }
        #[cfg(not(feature = "builtin-commands"))]
        {
            Self::empty()
        }
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
        Self::default().add_typed_commands(names.into_iter().map(|name| (name, CommandType::Builtin)))
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

    /// Whether `name` (with words separated by single spaces) is a known command.
    #[inline]
    pub fn is_known(&self, name: &str) -> bool {
        self.commands.all.names.contains(name)
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

impl Clone for CommandSet {
    fn clone(&self) -> Self {
        Self { names: self.names.clone(), prefixes: self.prefixes.clone() }
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
    let block = match lex(source, 0, LexOptions::BLOCK) {
        Ok(tokens) => parse_pipelines::parse_block(tokens::Tokens::from_lexed(&working_set, &tokens), whole_source),
        Err(diagnostic) => {
            working_set.error(diagnostic);
            Block { span: whole_source, pipelines: Vec::new() }
        }
    };
    let Collected { comments, ignored, parse_errors } = working_set.into_collected();
    (Ast { source, block, comments, shebang, ignored }, parse_errors)
}

/// Parse the block covering `span` of `source` statement by statement (see
/// [`crate::parse_block_streaming`]).
pub(crate) fn parse_block_streaming<'a>(
    source: &'a str,
    span: Span,
    lookup: &'a dyn CommandLookup,
    sink: &mut dyn BlockSink<'a>,
) -> Result<(), Vec<Diagnostic>> {
    let working_set = WorkingSet::with_lookup(source, lookup);
    let tokens = lex(span.slice(source), span.start, LexOptions::BLOCK).map_err(|diagnostic| vec![diagnostic])?;
    parse_pipelines::parse_block_streaming(tokens::Tokens::from_lexed(&working_set, &tokens), sink)
}
