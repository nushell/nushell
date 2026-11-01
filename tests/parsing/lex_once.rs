//! Lexing with bracket tables ([`StateWorkingSet::lex_once`]) must not change what the parser
//! produces.
//!
//! Every `.nu` file of the standard library, the default config files and the toolkit is parsed
//! twice in the shell's engine, with [`StateWorkingSet::lex_once`] off and on, and everything
//! the parse produced is compared. The lexer itself is compared on hand-written inputs and
//! variants of them by the tests in `crates/nu-parser/src/lex_once.rs`.

use nu_parser::{FlatShape, flatten_block};
use nu_protocol::{DeclId, Span, VarId, ast::Block, engine::StateWorkingSet};
use nu_test_support::{fs::nu_files, prelude::*, tester::parse_file};

/// The smallest file that gets a bracket table (`BRACKET_TABLE_LEN` in
/// `crates/nu-parser/src/lex_once.rs`). Smaller inputs are padded to it with trailing spaces,
/// which lex the same with and without tables.
const MIN_TABLE_LEN: usize = 1024;

/// Everything a parse produces, in a form that compares equal for equal parses.
#[derive(Debug, PartialEq)]
struct ParseResult {
    /// The parsed block, then every block the parse added. `Block::ir_block` is left out: it is
    /// compiled from the blocks compared here, and because `Call::parser_info` is a `HashMap`, the
    /// order in which the IR pushes parser info differs between any two parses. Comparing the
    /// blocks as JSON makes `parser_info` compare as a map.
    blocks: Vec<serde_json::Value>,
    /// Every variable the parse added.
    variables: Vec<String>,
    /// The signature of every declaration the parse added.
    declarations: Vec<serde_json::Value>,
    spans: Vec<Span>,
    parse_errors: Vec<ParseError>,
    parse_warnings: Vec<String>,
    compile_errors: Vec<CompileError>,
    shapes: Vec<(Span, FlatShape)>,
}

impl ParseResult {
    /// What the parse of `block` left in `working_set`.
    fn new(working_set: &StateWorkingSet, block: &Block) -> Self {
        let without_ir = |block: &Block| {
            let mut block = block.clone();
            block.ir_block = None;
            let mut json = serde_json::to_value(block).expect("blocks serialize to JSON");
            sort_hidden_sets(&mut json);
            json
        };
        let permanent = working_set.permanent_state;
        ParseResult {
            blocks: std::iter::once(block)
                .chain(working_set.delta.blocks.iter().map(|block| &**block))
                .map(without_ir)
                .collect(),
            variables: (permanent.num_vars()..working_set.num_vars())
                .map(|id| format!("{:?}", working_set.get_variable(VarId::new(id))))
                .collect(),
            declarations: (permanent.num_decls()..working_set.num_decls())
                .map(|id| {
                    let signature = working_set.get_decl(DeclId::new(id)).signature();
                    serde_json::to_value(signature).expect("signatures serialize to JSON")
                })
                .collect(),
            spans: working_set.delta.spans.clone(),
            parse_errors: working_set.parse_errors.clone(),
            parse_warnings: working_set
                .parse_warnings
                .iter()
                .map(|warning| format!("{warning:?}"))
                .collect(),
            compile_errors: working_set.compile_errors.clone(),
            shapes: flatten_block(working_set, block),
        }
    }
}

/// `ImportPattern::hidden` is a `HashSet`, which serializes in its iteration order, and that
/// differs between any two parses: sort it. (`Call::parser_info`, the only other hashed collection
/// in the AST, is a map and serializes as a JSON object, which compares as a map.)
fn sort_hidden_sets(json: &mut serde_json::Value) {
    match json {
        serde_json::Value::Object(fields) => {
            for (name, field) in fields.iter_mut() {
                if name == "hidden"
                    && let serde_json::Value::Array(items) = field
                {
                    items.sort_by_key(|item| item.to_string());
                }
                sort_hidden_sets(field);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(sort_hidden_sets),
        _ => {}
    }
}

#[test]
fn lex_once_parses_like_scanning() -> Result {
    let engine_state = test().engine_state;
    let mut files = vec![];
    for dir in [
        "crates/nu-std/std",
        "crates/nu-config/default_files",
        "toolkit",
    ] {
        let found = nu_files(WORKSPACE_ROOT.join(dir));
        assert!(!found.is_empty(), "no .nu files under {dir}");
        files.extend(found);
    }

    for path in files {
        // Every input is at least as large as the smallest file that gets a bracket table, so the
        // parse with `lex_once` on records and uses one (a unit test of `lex_file` in nu-parser
        // checks that it does).
        let mut source = std::fs::read(&path).expect("file is readable");
        if source.len() < MIN_TABLE_LEN {
            source.resize(MIN_TABLE_LEN, b' ');
        }
        let (working_set, block) = parse_file(&engine_state, &path, &source, false);
        let scanning = ParseResult::new(&working_set, &block);
        let (working_set, block) = parse_file(&engine_state, &path, &source, true);
        // A bracket table only serves the parse of its file.
        assert!(
            working_set.bracket_tables.is_empty(),
            "the parse of {} kept its bracket tables",
            path.display()
        );
        let jumping = ParseResult::new(&working_set, &block);
        assert!(
            scanning == jumping,
            "bracket tables changed the parse of {}",
            path.display()
        );
    }
    Ok(())
}
