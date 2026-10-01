//! Lexing with bracket tables ([`StateWorkingSet::lex_once`]) must not change what the parser
//! produces.
//!
//! Every `.nu` file of the standard library, the default config files and the toolkit is parsed
//! twice in the shell's engine, with [`StateWorkingSet::lex_once`] off and on, and everything
//! the parse produced is compared. The lexer itself is compared on hand-written inputs and
//! variants of them by the tests in `crates/nu-parser/src/lex_once.rs`.

use nu_parser::{FlatShape, flatten_block, parse};
use nu_protocol::{
    DeclId, Span, VarId,
    ast::Block,
    engine::{EngineState, StateWorkingSet},
};
use nu_test_support::prelude::*;
use std::path::{Path, PathBuf};

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

/// Parse `source` as the contents of the file at `path`, the way `source` parses a file.
fn parse_file(
    engine_state: &EngineState,
    path: &Path,
    source: &[u8],
    lex_once: bool,
) -> ParseResult {
    let mut working_set = StateWorkingSet::new(engine_state);
    working_set.lex_once = lex_once;
    working_set
        .files
        .push(path.to_path_buf(), Span::unknown())
        .expect("a single file cannot be a circular import");
    let block = parse(
        &mut working_set,
        Some(&path.to_string_lossy()),
        source,
        false,
    );
    working_set.files.pop();

    let without_ir = |block: &Block| {
        let mut block = block.clone();
        block.ir_block = None;
        let mut json = serde_json::to_value(block).expect("blocks serialize to JSON");
        sort_hidden_sets(&mut json);
        json
    };
    let permanent = working_set.permanent_state;
    ParseResult {
        blocks: std::iter::once(&block)
            .chain(&working_set.delta.blocks)
            .map(|block| without_ir(block))
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
        shapes: flatten_block(&working_set, &block),
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

/// Every `.nu` file under `dir`, in a stable order.
fn nu_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("directory is readable")
        .map(|entry| entry.expect("directory entry is readable").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            nu_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "nu") {
            files.push(path);
        }
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
        let found = files.len();
        nu_files(&WORKSPACE_ROOT.join(dir), &mut files);
        assert!(files.len() > found, "no .nu files under {dir}");
    }

    for path in files {
        let source = std::fs::read(&path).expect("file is readable");
        let scanning = parse_file(&engine_state, &path, &source, false);
        let jumping = parse_file(&engine_state, &path, &source, true);
        assert!(
            scanning == jumping,
            "bracket tables changed the parse of {}",
            path.display()
        );
    }
    Ok(())
}
