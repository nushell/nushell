//! The grammar in `devdocs/grammar` must stay true to nu-parser.
//!
//! `grammar.md` holds the grammar in its ```` ```ebnf ```` fences, and `gen-grammar.nu` derives
//! `grammar.bnf` and `grammar.ebnf` from them. These tests check that
//!
//! - both derived files are up to date ([`grammar_files_are_generated_from_grammar_md`]) and
//!   define the same rules ([`bnf_and_ebnf_define_the_same_rules`]);
//! - every rule the EBNF refers to is defined once, and every rule can be reached from the start
//!   symbols of the grammar's two layers, `source_file` and `block_tokens`
//!   ([`ebnf_rules_are_defined_and_reachable`]);
//! - every `; nu:` annotation names files and items that exist
//!   ([`annotations_name_existing_code`]);
//! - the text of every value that nu-parser parses in the accepted language fixtures, the standard
//!   library, the default config files and the toolkit matches the grammar's rule for that kind
//!   of value ([`grammar_matches_parsed_values`]).
//!
//! The last test runs the EBNF rules against the text. Much of the grammar can't be run: rules
//! stated in words (`? ... ?` special sequences) depend on the command signatures or on the
//! context the lexer is in. A match that needs one of them is not decided and is not checked; the
//! special sequences that are plain byte classes are ([`class`]).

use super::language::FIXTURES;
use fancy_regex::Regex;
use nu_protocol::{
    Span,
    ast::{Expr, Expression, Traverse, Unit},
};
use nu_test_support::{fs::nu_files, prelude::*, tester::parse_file};
use pretty_assertions::assert_eq;
use rustc_hash::FxHashMap;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::LazyLock,
};

/// The grammar files, relative to the workspace root.
const GRAMMAR: &str = "devdocs/grammar";

/// A definition of the EBNF grammar (ISO 14977, as `gen-grammar.nu` writes it).
#[derive(Debug)]
enum Node {
    /// A quoted terminal: these bytes.
    Terminal(Vec<u8>),
    /// A `? ... ?` special sequence that is a byte class ([`class`]): how many bytes of the
    /// input's start it matches.
    Class(Class),
    /// Any other `? ... ?` special sequence: a rule stated in words.
    Special,
    /// Another rule, by its index in [`Grammar::names`].
    Rule(usize),
    /// `a, b, c`
    Seq(Vec<Node>),
    /// `a | b | c`
    Alt(Vec<Node>),
    /// `[ a ]`
    Optional(Box<Node>),
    /// `{ a }`
    Repeat(Box<Node>),
    /// `N * a`
    Times(usize, Box<Node>),
    /// `a - b`: what `a` matches unless `b` matches it too.
    Except(Box<Node>, Box<Node>),
}

/// The rules of `grammar.ebnf`.
struct Grammar {
    /// The name of every rule the grammar defines or refers to.
    names: Vec<String>,
    index: HashMap<String, usize>,
    /// The definition of each rule of `names`, or `None` for a rule the grammar never defines.
    definitions: Vec<Option<Node>>,
    /// The rules in the order the grammar defines them; a rule defined twice is here twice.
    order: Vec<usize>,
}

/// A token of `grammar.ebnf`.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    Number(usize),
    Terminal(Vec<u8>),
    Special(String),
    Punct(u8),
}

/// The bytes the text of a terminal stands for: `\n`, `\r` and `\t` are the line feed, carriage
/// return and tab, and any other text is itself, backslashes included (section 0 of grammar.md).
fn terminal_bytes(text: &[u8]) -> Vec<u8> {
    match text {
        br"\n" => b"\n".to_vec(),
        br"\r" => b"\r".to_vec(),
        br"\t" => b"\t".to_vec(),
        text => text.to_vec(),
    }
}

/// The tokens of `text`, without its `(* ... *)` comments.
fn tokenize(text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut tokens = vec![];
    let mut i = 0;
    let until = |from: usize, end: &[u8]| -> usize {
        (from..bytes.len())
            .find(|&j| bytes[j..].starts_with(end))
            .unwrap_or_else(|| {
                panic!(
                    "unterminated {:?} at byte {from}",
                    String::from_utf8_lossy(end)
                )
            })
    };
    while i < bytes.len() {
        match bytes[i] {
            b if b.is_ascii_whitespace() => i += 1,
            b'(' if bytes[i..].starts_with(b"(*") => i = until(i + 2, b"*)") + 2,
            quote @ (b'"' | b'\'') => {
                let end = until(i + 1, &[quote]);
                tokens.push(Token::Terminal(terminal_bytes(&bytes[i + 1..end])));
                i = end + 1;
            }
            b'?' => {
                let end = until(i + 1, b"?");
                tokens.push(Token::Special(text[i + 1..end].trim().to_string()));
                i = end + 1;
            }
            b if b.is_ascii_alphanumeric() || b == b'_' => {
                let end = (i..bytes.len())
                    .find(|&j| !(bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_'))
                    .unwrap_or(bytes.len());
                let word = &text[i..end];
                tokens.push(match word.parse() {
                    Ok(number) => Token::Number(number),
                    Err(_) => Token::Name(word.to_string()),
                });
                i = end;
            }
            b => {
                tokens.push(Token::Punct(b));
                i += 1;
            }
        }
    }
    tokens
}

/// A recursive descent parser of the token list of `grammar.ebnf`.
struct EbnfParser {
    tokens: Vec<Token>,
    pos: usize,
    names: Vec<String>,
    index: HashMap<String, usize>,
}

impl EbnfParser {
    /// The index of the rule `name`.
    fn rule(&mut self, name: String) -> usize {
        let next = self.names.len();
        let index = *self.index.entry(name.clone()).or_insert(next);
        if index == next {
            self.names.push(name);
        }
        index
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, punct: u8) -> bool {
        let found = self.peek() == Some(&Token::Punct(punct));
        self.pos += usize::from(found);
        found
    }

    fn expect(&mut self, punct: u8) {
        assert!(
            self.eat(punct),
            "expected {:?}, found {:?} (token {})",
            punct as char,
            self.peek(),
            self.pos
        );
    }

    /// `definitions_list = single_definition, { "|", single_definition }`
    fn alternatives(&mut self) -> Node {
        let mut alternatives = vec![self.sequence()];
        while self.eat(b'|') {
            alternatives.push(self.sequence());
        }
        match alternatives.len() {
            1 => alternatives.remove(0),
            _ => Node::Alt(alternatives),
        }
    }

    /// `single_definition = syntactic_term, { ",", syntactic_term }`
    fn sequence(&mut self) -> Node {
        let mut terms = vec![self.term()];
        while self.eat(b',') {
            terms.push(self.term());
        }
        match terms.len() {
            1 => terms.remove(0),
            _ => Node::Seq(terms),
        }
    }

    /// `syntactic_term = syntactic_factor, [ "-", syntactic_exception ]`
    fn term(&mut self) -> Node {
        let factor = self.factor();
        match self.eat(b'-') {
            true => Node::Except(Box::new(factor), Box::new(self.factor())),
            false => factor,
        }
    }

    /// `syntactic_factor = [ integer, "*" ], syntactic_primary`; an empty primary matches the
    /// empty string.
    fn factor(&mut self) -> Node {
        if let Some(&Token::Number(times)) = self.peek() {
            self.pos += 1;
            self.expect(b'*');
            return Node::Times(times, Box::new(self.primary()));
        }
        self.primary()
    }

    fn primary(&mut self) -> Node {
        let token = self.peek().cloned();
        match token {
            Some(Token::Punct(open @ (b'[' | b'{' | b'('))) => {
                self.pos += 1;
                let inner = self.alternatives();
                let close = match open {
                    b'[' => b']',
                    b'{' => b'}',
                    _ => b')',
                };
                self.expect(close);
                match open {
                    b'[' => Node::Optional(Box::new(inner)),
                    b'{' => Node::Repeat(Box::new(inner)),
                    _ => inner,
                }
            }
            Some(Token::Name(name)) => {
                self.pos += 1;
                Node::Rule(self.rule(name))
            }
            Some(Token::Terminal(bytes)) => {
                self.pos += 1;
                Node::Terminal(bytes)
            }
            Some(Token::Special(text)) => {
                self.pos += 1;
                match class(&text) {
                    Some(class) => Node::Class(class),
                    None => Node::Special,
                }
            }
            _ => Node::Seq(vec![]),
        }
    }
}

impl Grammar {
    fn parse(text: &str) -> Self {
        let mut parser = EbnfParser {
            tokens: tokenize(text),
            pos: 0,
            names: vec![],
            index: HashMap::new(),
        };
        let mut definitions = vec![];
        let mut order = vec![];
        while let Some(token) = parser.peek().cloned() {
            let Token::Name(name) = token else {
                panic!(
                    "expected a rule name, found {token:?} (token {})",
                    parser.pos
                );
            };
            parser.pos += 1;
            let rule = parser.rule(name);
            parser.expect(b'=');
            let definition = parser.alternatives();
            parser.expect(b';');
            definitions.resize_with(parser.names.len(), || None);
            definitions[rule].get_or_insert(definition);
            order.push(rule);
        }
        definitions.resize_with(parser.names.len(), || None);
        Grammar {
            names: parser.names,
            index: parser.index,
            definitions,
            order,
        }
    }

    /// The index of the rule `name`, which the grammar must define.
    fn rule(&self, name: &str) -> usize {
        let rule = self.index.get(name).copied();
        rule.filter(|&rule| self.definitions[rule].is_some())
            .unwrap_or_else(|| panic!("the grammar defines no rule {name}"))
    }
}

/// A byte class: how many bytes at the start of an input it matches, if it matches.
type Class = fn(&[u8]) -> Option<usize>;

/// The byte classes among the special sequences, by their exact text. The other special
/// sequences state context the grammar cannot express, and a match that reaches one of them is
/// not decided.
fn class(special: &str) -> Option<Class> {
    fn byte_not_in(set: &'static [u8]) -> impl Fn(&[u8]) -> Option<usize> {
        move |input| input.first().filter(|b| !set.contains(b)).map(|_| 1)
    }
    Some(match special {
        "any byte" => |input| input.first().map(|_| 1),
        r#"any byte except "\n""# => |input| byte_not_in(b"\n")(input),
        r#"byte except "'""# | r#"any byte except "'""# => |input| byte_not_in(b"'")(input),
        r#"byte except "`""# => |input| byte_not_in(b"`")(input),
        r#"byte except "\"" and "\\""# => |input| byte_not_in(b"\"\\")(input),
        r#"any byte except "\""# => |input| byte_not_in(b"\\")(input),
        r#"any byte except "." "[" "(" "{" "+" "-" "*" "^" "%" "/" "=" "!" "<" ">" "&" "|""# => {
            |input| byte_not_in(b".[({+-*^%/=!<>&|")(input)
        }
        r#"a byte in "A".."Z" or "a".."z""# => {
            |input| input.first().filter(|b| b.is_ascii_alphabetic()).map(|_| 1)
        }
        "one UTF-8 encoded character" => |input| {
            let len = match input.first()? {
                0x00..=0x7f => 1,
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf7 => 4,
                _ => return None,
            };
            std::str::from_utf8(input.get(..len)?).ok().map(|_| len)
        },
        "the empty string" => |_| Some(0),
        _ => return None,
    })
}

/// Matches the rules of a [`Grammar`] against an input, finding every way each can match.
struct Matcher<'a> {
    grammar: &'a Grammar,
    input: &'a [u8],
    /// Where each rule matched from a position can end, by `(rule, position)`.
    memo: FxHashMap<(usize, usize), Vec<usize>>,
    /// By start position, the rules being matched from there.
    active: Vec<Vec<usize>>,
    /// Whether the match reached a special sequence stated in words.
    undecided: bool,
}

impl<'a> Matcher<'a> {
    fn new(grammar: &'a Grammar, input: &'a [u8]) -> Self {
        Matcher {
            grammar,
            input,
            memo: FxHashMap::default(),
            active: vec![vec![]; input.len() + 1],
            undecided: false,
        }
    }

    /// Every position where a match of `rule` starting at `pos` can end, in order.
    fn rule_ends(&mut self, rule: usize, pos: usize) -> Vec<usize> {
        if let Some(ends) = self.memo.get(&(rule, pos)) {
            return ends.clone();
        }
        // A left-recursive reference matches nothing more.
        if self.active[pos].contains(&rule) {
            return vec![];
        }
        let grammar = self.grammar;
        let Some(definition) = &grammar.definitions[rule] else {
            panic!("rule {} is not defined", grammar.names[rule]);
        };
        self.active[pos].push(rule);
        let ends = self.ends(definition, pos);
        self.active[pos].pop();
        self.memo.insert((rule, pos), ends.clone());
        ends
    }

    /// Every position where a match of `node` starting at one of `starts` can end, in order.
    fn ends_from(&mut self, node: &'a Node, starts: Vec<usize>) -> Vec<usize> {
        let mut ends: Vec<usize> = starts
            .into_iter()
            .flat_map(|start| self.ends(node, start))
            .collect();
        ends.sort_unstable();
        ends.dedup();
        ends
    }

    /// Every position where a match of `node` starting at `pos` can end, in order.
    fn ends(&mut self, node: &'a Node, pos: usize) -> Vec<usize> {
        match node {
            Node::Terminal(bytes) => (self.input[pos..].starts_with(bytes))
                .then_some(pos + bytes.len())
                .into_iter()
                .collect(),
            Node::Class(class) => class(&self.input[pos..])
                .map(|len| pos + len)
                .into_iter()
                .collect(),
            Node::Special => {
                self.undecided = true;
                vec![]
            }
            Node::Rule(rule) => self.rule_ends(*rule, pos),
            Node::Seq(items) => items
                .iter()
                .fold(vec![pos], |starts, item| self.ends_from(item, starts)),
            Node::Alt(alternatives) => {
                let mut ends: Vec<usize> = (alternatives.iter())
                    .flat_map(|alternative| self.ends(alternative, pos))
                    .collect();
                ends.sort_unstable();
                ends.dedup();
                ends
            }
            Node::Optional(inner) => {
                let mut ends = self.ends(inner, pos);
                if let Err(at) = ends.binary_search(&pos) {
                    ends.insert(at, pos);
                }
                ends
            }
            Node::Repeat(inner) => {
                let mut reached = vec![false; self.input.len() + 1];
                reached[pos] = true;
                let mut frontier = vec![pos];
                while let Some(start) = frontier.pop() {
                    for end in self.ends(inner, start) {
                        if !std::mem::replace(&mut reached[end], true) {
                            frontier.push(end);
                        }
                    }
                }
                (pos..reached.len()).filter(|&end| reached[end]).collect()
            }
            Node::Times(times, inner) => {
                (0..*times).fold(vec![pos], |starts, _| self.ends_from(inner, starts))
            }
            Node::Except(node, exception) => {
                let mut ends = self.ends(node, pos);
                let excluded = self.ends(exception, pos);
                ends.retain(|end| excluded.binary_search(end).is_err());
                ends
            }
        }
    }

    /// Whether `rule` matches the whole input, or `None` if that depends on a special sequence
    /// stated in words.
    fn matches(mut self, rule: &str) -> Option<bool> {
        let ends = self.rule_ends(self.grammar.rule(rule), 0);
        let whole = ends.contains(&self.input.len());
        (whole || !self.undecided).then_some(whole)
    }
}

/// The rules of `grammar.ebnf`.
fn ebnf() -> Grammar {
    let path = WORKSPACE_ROOT.join(GRAMMAR).join("grammar.ebnf");
    Grammar::parse(&std::fs::read_to_string(path).expect("grammar.ebnf is readable"))
}

/// Every rule `node` refers to.
fn references(node: &Node, out: &mut Vec<usize>) {
    match node {
        Node::Terminal(_) | Node::Class(_) | Node::Special => {}
        Node::Rule(rule) => out.push(*rule),
        Node::Seq(nodes) | Node::Alt(nodes) => nodes.iter().for_each(|node| references(node, out)),
        Node::Optional(node) | Node::Repeat(node) | Node::Times(_, node) => references(node, out),
        Node::Except(node, exception) => {
            references(node, out);
            references(exception, out);
        }
    }
}

#[test]
#[deps(NU)]
fn grammar_files_are_generated_from_grammar_md() -> Result {
    let result: CompleteResult = test()
        .cwd(GRAMMAR)
        .run("nu -n gen-grammar.nu --check | complete")?;
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    Ok(())
}

#[test]
fn bnf_and_ebnf_define_the_same_rules() -> Result {
    let bnf = std::fs::read_to_string(WORKSPACE_ROOT.join(GRAMMAR).join("grammar.bnf"))?;
    // A rule starts a line: `<name> ::= ...`; the EBNF spells `<a-b>` as `a_b`.
    let bnf_rules: Vec<String> = (bnf.lines())
        .filter_map(|line| {
            let (name, rest) = line.strip_prefix('<')?.split_once('>')?;
            rest.trim_start()
                .starts_with("::=")
                .then(|| name.replace('-', "_"))
        })
        .collect();
    let grammar = ebnf();
    let ebnf_rules: Vec<String> = (grammar.order.iter())
        .map(|&rule| grammar.names[rule].clone())
        .collect();
    assert_eq!(bnf_rules, ebnf_rules);
    Ok(())
}

#[test]
fn ebnf_rules_are_defined_and_reachable() {
    let grammar = ebnf();
    let mut defined = HashSet::new();
    let duplicates: Vec<&str> = (grammar.order.iter())
        .filter(|&&rule| !defined.insert(rule))
        .map(|&rule| grammar.names[rule].as_str())
        .collect();
    assert!(
        duplicates.is_empty(),
        "rules defined more than once: {duplicates:?}"
    );
    let undefined: Vec<&str> = (0..grammar.names.len())
        .filter(|&rule| grammar.definitions[rule].is_none())
        .map(|rule| grammar.names[rule].as_str())
        .collect();
    assert!(
        undefined.is_empty(),
        "rules referred to but never defined: {undefined:?}"
    );

    // The grammar has two layers, each with its start symbol: the lexer turns a `source_file`
    // (the first rule) into tokens, and the lite parse groups the tokens into `block_tokens`.
    assert_eq!(grammar.names[grammar.order[0]], "source_file");
    let starts = ["source_file", "block_tokens"].map(|name| grammar.rule(name));
    let mut reached = BTreeSet::from(starts);
    let mut pending = starts.to_vec();
    while let Some(rule) = pending.pop() {
        let mut used = vec![];
        if let Some(definition) = &grammar.definitions[rule] {
            references(definition, &mut used);
        }
        pending.extend(used.into_iter().filter(|&used| reached.insert(used)));
    }
    let unreachable: Vec<&str> = (grammar.order.iter())
        .filter(|rule| !reached.contains(rule))
        .map(|&rule| grammar.names[rule].as_str())
        .collect();
    assert!(
        unreachable.is_empty(),
        "rules that neither source_file nor block_tokens reach: {unreachable:?}"
    );
}

#[test]
fn annotations_name_existing_code() -> Result {
    static REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"([A-Za-z0-9_/]+\.rs)(?:::([A-Za-z0-9_:]+))?").expect("valid regex")
    });
    let bnf = std::fs::read_to_string(WORKSPACE_ROOT.join(GRAMMAR).join("grammar.bnf"))?;
    let lines: Vec<&str> = bnf.lines().collect();
    let mut references = 0;
    let mut missing = vec![];
    for (index, line) in lines.iter().enumerate() {
        let Some(first) = line.strip_prefix("; nu: ") else {
            continue;
        };
        // An annotation continues on the comment lines indented under it.
        let rest = lines[index + 1..]
            .iter()
            .map_while(|line| line.strip_prefix(";     "));
        let annotation = std::iter::once(first)
            .chain(rest)
            .collect::<Vec<_>>()
            .join(" ");
        // `;` separates the parts of an annotation. A part that starts with a crate's name
        // (`nu-cmd-lang core_commands/if_.rs`) names files of that crate, the others name files
        // of nu-parser (`parse_calls.rs::parse_call`).
        for part in annotation.split(';') {
            let krate = (part.split_whitespace().next())
                .filter(|word| word.starts_with("nu-"))
                .unwrap_or("nu-parser");
            for captures in REFERENCE.captures_iter(part) {
                let captures = captures.expect("the reference regex cannot fail");
                references += 1;
                let path = WORKSPACE_ROOT
                    .join("crates")
                    .join(krate)
                    .join("src")
                    .join(&captures[1]);
                let Ok(source) = std::fs::read_to_string(&path) else {
                    missing.push(format!("line {}: no file {}", index + 1, path.display()));
                    continue;
                };
                // `LitePipeline::push` names `push`.
                let Some(item) = captures
                    .get(2)
                    .and_then(|item| item.as_str().rsplit("::").next())
                else {
                    continue;
                };
                let definition =
                    format!(r"\b(fn|struct|enum|const|static|trait|type|impl|mod)\s+{item}\b");
                let defined = Regex::new(&definition)
                    .expect("valid regex")
                    .is_match(&source);
                if !defined.expect("the definition regex cannot fail") {
                    missing.push(format!(
                        "line {}: no `{item}` in {}",
                        index + 1,
                        path.display()
                    ));
                }
            }
        }
    }
    assert!(references > 100, "the annotations are all found");
    assert!(
        missing.is_empty(),
        "grammar.bnf names code that does not exist:\n{}",
        missing.join("\n")
    );
    Ok(())
}

/// The rule of the grammar that the text of `expr` must match, and the text after the rewriting
/// the rule's annotation describes if it has one, or `None` for expressions this test does not
/// check.
///
/// The parser also makes expressions that have no text of their own, whose span covers other
/// text: the `$it` of a row condition (`where size > 1`), the record and the closure of an
/// environment shorthand (`FOO=1 ls`) and the block of a `let` value. An expression of those kinds
/// is checked only when it starts with the first byte of its rule.
fn rule_for(expr: &Expr, text: &[u8]) -> Option<(&'static str, Option<Vec<u8>>)> {
    let starts_with = |byte: u8| text.first() == Some(&byte);
    // `<int>` and `<float>` are matched "after deleting every `_`".
    let without_underscores =
        || -> Vec<u8> { text.iter().copied().filter(|&byte| byte != b'_').collect() };
    let rule = match expr {
        Expr::Int(_) => return Some(("int", Some(without_underscores()))),
        // "inf", "infinity" and "nan" are case-insensitive, and the exponent may be "e" or "E".
        Expr::Float(_) => {
            return Some(("float", Some(without_underscores().to_ascii_lowercase())));
        }
        // A filesize unit is "matched against the ASCII-uppercased item".
        Expr::ValueWithUnit(value) if matches!(value.unit.item, Unit::Filesize(_)) => {
            return Some(("filesize", Some(text.to_ascii_uppercase())));
        }
        Expr::ValueWithUnit(_) => "duration",
        Expr::Bool(_) => "bool",
        Expr::Nothing => "nothing",
        Expr::DateTime(_) => "datetime",
        Expr::Binary(_) => "binary",
        Expr::RawString(_) => "raw_string",
        Expr::String(_) | Expr::GlobPattern(..) | Expr::Filepath(..) | Expr::Directory(..) => {
            "string"
        }
        Expr::StringInterpolation(_) => "interpolation",
        Expr::Range(_) => "range",
        Expr::CellPath(_) => "cell_path_literal_body",
        Expr::FullCellPath(_) => "full_cell_path",
        Expr::Var(_) if starts_with(b'$') => "variable",
        Expr::List(_) | Expr::Table(_) if starts_with(b'[') => "list_or_table",
        Expr::Record(_) if starts_with(b'{') => "record",
        Expr::Closure(_) if starts_with(b'{') => "closure",
        Expr::Block(_) if starts_with(b'{') => "block",
        Expr::Subexpression(_) if starts_with(b'(') => "paren_expr",
        _ => return None,
    };
    Some((rule, None))
}

/// For [`Traverse::flat_map`]: every expression.
fn itself(expr: &Expression) -> Vec<&Expression> {
    vec![expr]
}

#[test]
fn grammar_matches_parsed_values() -> Result {
    let grammar = ebnf();
    let engine_state = test().engine_state;
    let mut files = vec![];
    for dir in [
        &format!("{FIXTURES}/accept"),
        "crates/nu-std/std",
        "crates/nu-config/default_files",
        "toolkit",
    ] {
        files.extend(nu_files(WORKSPACE_ROOT.join(dir)));
    }

    // The rules this test met, and how many values each one decided.
    let mut decided: BTreeMap<&str, usize> = BTreeMap::new();
    let mut mismatches = vec![];
    for path in &files {
        let source = std::fs::read(path)?;
        let (working_set, block) = parse_file(&engine_state, path, &source, true);
        let file = block.span.expect("a parsed file has a span");
        let mut expressions = vec![];
        block.flat_map(&working_set, &itself, &mut expressions);
        // The number of a unit value is its `<unit-number>`, not an `<int>` or `<float>`.
        let unit_numbers: Vec<Span> = (expressions.iter())
            .filter_map(|expr| match &expr.expr {
                Expr::ValueWithUnit(value) => Some(value.expr.span),
                _ => None,
            })
            .collect();
        for expr in expressions {
            if !file.contains_span(expr.span) || unit_numbers.contains(&expr.span) {
                continue;
            }
            let text = working_set.get_span_contents(expr.span);
            let Some((rule, rewritten)) = rule_for(&expr.expr, text) else {
                continue;
            };
            let checked = rewritten.as_deref().unwrap_or(text);
            let matched = Matcher::new(&grammar, checked).matches(rule);
            let count = decided.entry(rule).or_default();
            match matched {
                Some(true) => *count += 1,
                Some(false) => mismatches.push(format!(
                    "{}: `{rule}` does not match {:?}",
                    path.display(),
                    String::from_utf8_lossy(rewritten.as_deref().unwrap_or(text))
                )),
                None => {}
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "the grammar rejects values nu-parser parsed:\n{}",
        mismatches.join("\n")
    );
    let never_decided: Vec<&str> = (decided.iter())
        .filter(|(_, count)| **count == 0)
        .map(|(rule, _)| *rule)
        .collect();
    assert!(
        never_decided.is_empty(),
        "the grammar decides no value of these rules: {never_decided:?}"
    );
    Ok(())
}
