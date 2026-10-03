//! The language fixtures pin what nu-parser makes of every construct of the language.
//!
//! [`FIXTURES`] holds one snippet per file: `accept/<area>/*.nu` are syntactically valid and
//! `reject/<area>/*.nu` are not. Beside each area directory, `<area>.golden` records what the
//! shell's engine makes of each of the area's snippets, parsed on its own: the shapes it
//! highlights, the errors and warnings the parse and the compiler report, and the IR of every
//! block. [`fixtures_parse_as_recorded`] fails on any difference. After an intended change,
//! regenerate the golden files and review their diff:
//!
//! ```text
//! NU_TEST_UPDATE_GOLDEN=1 cargo test --test tests -- parsing::language
//! ```
//!
//! Golden files hold no offsets or ids of the engine, so that adding a command or changing the
//! standard library does not change them: spans count from the start of the snippet, the IR names
//! commands without their ids, numbers the snippet's own variables and blocks in the order the
//! parse added them (`#0`, `#1`, ...), and shows only the name of the parser info a call pushes.

use fancy_regex::{Captures, Regex};
use miette::Diagnostic;
use nu_parser::flatten_block;
use nu_protocol::{LAST_VARIABLE_ID, Span, VarId, ast::Block, engine::EngineState};
use nu_test_support::{fs::nu_files, prelude::*, tester::parse_file};
use pretty_assertions::StrComparison;
use std::{
    collections::HashMap,
    fmt::Write,
    path::Path,
    sync::{Arc, LazyLock},
};

/// The language fixtures, relative to the workspace root.
pub(super) const FIXTURES: &str = "crates/nu-parser/tests/fixtures/language";

/// Set this environment variable to write the golden files instead of comparing with them.
const UPDATE_GOLDEN: &str = "NU_TEST_UPDATE_GOLDEN";

/// The golden file entry of the snippet at `path`, named `name` in the entry's header.
fn render(engine_state: &EngineState, path: &Path, name: &str) -> String {
    let source = std::fs::read(path).expect("fixture is readable");
    let (working_set, block) = parse_file(engine_state, path, &source, true);
    // The snippet's file comes after every file the engine already holds.
    let file = block.span.expect("a parsed file has a span");
    let at = |span: Span| match file.contains_span(span) {
        true => format!(
            "{}..{} {:?}",
            span.start - file.start,
            span.end - file.start,
            String::from_utf8_lossy(working_set.get_span_contents(span)),
        ),
        false => "outside the snippet".into(),
    };

    let mut out = format!("=== {name}\n");
    for (span, shape) in flatten_block(&working_set, &block) {
        writeln!(out, "shape {shape} {}", at(span)).expect("writing to a String");
    }
    let diagnostics = (working_set.parse_errors.iter())
        .map(|error| ("error", error as &dyn Diagnostic))
        .chain((working_set.parse_warnings.iter()).map(|w| ("warning", w as &dyn Diagnostic)))
        .chain(
            (working_set.compile_errors.iter()).map(|e| ("compile error", e as &dyn Diagnostic)),
        );
    for (kind, diagnostic) in diagnostics {
        let code = diagnostic.code().map(|code| code.to_string());
        // An error of parse-time evaluation carries the whole report of the shell error as its
        // message, drawn for the terminal the test runs in; its first line names the error.
        let message = diagnostic.to_string();
        let message = message.lines().next().unwrap_or_default();
        writeln!(out, "{kind} {}: {message}", code.unwrap_or_default())
            .expect("writing to a String");
        for label in diagnostic.labels().into_iter().flatten() {
            let span = Span::new(label.offset(), label.offset() + label.len());
            writeln!(
                out,
                "  label {}: {}",
                at(span),
                label.label().unwrap_or_default()
            )
            .expect("writing to a String");
        }
        if let Some(help) = diagnostic.help() {
            writeln!(out, "  help: {help}").expect("writing to a String");
        }
    }

    // The snippet's own blocks and variables, numbered in the order the parse added them. Those
    // of the modules it loads (the standard library's, say) lie in other files and are left out,
    // so that changing those modules doesn't renumber the snippet's. A block or variable the parser
    // makes up (a row condition, say) has no span and counts as the snippet's.
    let first_block = engine_state.num_blocks();
    let own_blocks: Vec<(usize, &Arc<Block>)> = (working_set.delta.blocks.iter().enumerate())
        .filter(|(_, block)| block.span.is_none_or(|span| file.contains_span(span)))
        .map(|(index, block)| (first_block + index, block))
        .collect();
    let names = Names {
        blocks: (own_blocks.iter().enumerate())
            .map(|(number, (id, _))| (*id, format!("#{number}")))
            .collect(),
        vars: (engine_state.num_vars()..working_set.num_vars())
            .filter(|&id| {
                let span = working_set.get_variable(VarId::new(id)).declaration_span;
                span == Span::unknown() || file.contains_span(span)
            })
            .enumerate()
            .map(|(number, id)| (id, format!("#{number}")))
            .collect(),
    };
    // The main block, then the snippet's own blocks that have a span in it.
    let blocks: Vec<(String, Arc<Block>)> = std::iter::once(("main".to_string(), block))
        .chain(
            (own_blocks.into_iter())
                .filter(|(_, block)| block.span.is_some())
                .map(|(id, block)| (names.blocks[&id].clone(), block.clone())),
        )
        .collect();
    let mut merged = engine_state.clone();
    match merged.merge_delta(working_set.render()) {
        Ok(()) => out.push_str(&render_ir(&merged, &names, &blocks)),
        Err(error) => writeln!(out, "no ir: {error}").expect("writing to a String"),
    }
    out
}

/// The names that the golden files give the snippet's own blocks and variables, by id.
struct Names {
    blocks: HashMap<usize, String>,
    vars: HashMap<usize, String>,
}

/// The IR of `blocks`, rendered with `merged` (the engine with the snippet's parse merged into it),
/// with the snippet's blocks and variables named as in `names` and no other ids of the engine.
fn render_ir(merged: &EngineState, names: &Names, blocks: &[(String, Arc<Block>)]) -> String {
    static DECL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"decl \d+ ").expect("valid regex"));
    static VAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"var (\d+)").expect("valid regex"));
    static BLOCK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(block|closure|row_condition)\((\d+)\)").expect("valid regex")
    });
    // The value of a parser info is an `Expression`, printed with its absolute spans and ids, so
    // only the name of the info is kept (and the comment of the instruction, if it has one).
    static PARSER_INFO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(\s*\d+: push-parser-info\s+"[^"]*"), .*?(\s+# (?:label|from)\(.*)?$"#)
            .expect("valid regex")
    });
    // The engine's own variables `$nu`, `$in`, `$env` and `$ans` keep their ids; any other id the
    // snippet didn't add belongs to the engine or to a module the snippet loads.
    let var_name = |id: usize| match names.vars.get(&id) {
        Some(name) => name.clone(),
        None if id <= LAST_VARIABLE_ID.get() => id.to_string(),
        None => "outside the snippet".into(),
    };
    let block_name = |id: usize| match names.blocks.get(&id) {
        Some(name) => name.clone(),
        None => "outside the snippet".into(),
    };
    let id = |caps: &Captures<'_, str>, group: usize| -> usize {
        caps[group].parse().expect("ids are numbers")
    };

    let mut out = String::new();
    for (name, block) in blocks {
        let Some(ir_block) = &block.ir_block else {
            continue;
        };
        let ir = ir_block.display(merged).to_string();
        let ir = DECL.replace_all(&ir, "decl ");
        let ir = VAR.replace_all(&ir, |caps: &Captures<'_, str>| {
            format!("var {}", var_name(id(caps, 1)))
        });
        let ir = BLOCK.replace_all(&ir, |caps: &Captures<'_, str>| {
            format!("{}({})", &caps[1], block_name(id(caps, 2)))
        });
        // `Call::parser_info` is a `HashMap`, so a call with several entries pushes them in a
        // different order in every process: sort each run of `push-parser-info` instructions,
        // keeping the instruction numbers in place.
        let mut lines: Vec<String> = (ir.lines())
            .map(|line| PARSER_INFO.replace(line, "$1$2").into_owned())
            .collect();
        let is_push = |line: &String| line.contains(": push-parser-info");
        for run in lines.chunk_by_mut(|a, b| is_push(a) && is_push(b)) {
            if run.len() < 2 {
                continue;
            }
            let (numbers, mut instructions): (Vec<String>, Vec<String>) = (run.iter())
                .filter_map(|line| line.split_once(": "))
                .map(|(number, instruction)| (number.to_string(), instruction.to_string()))
                .unzip();
            instructions.sort();
            for (line, (number, instruction)) in
                run.iter_mut().zip(numbers.iter().zip(instructions))
            {
                *line = format!("{number}: {instruction}");
            }
        }
        writeln!(out, "ir {name}").expect("writing to a String");
        for line in lines {
            writeln!(out, "  {line}").expect("writing to a String");
        }
    }
    out
}

/// The `=== name` entries of a golden file.
fn entries(golden: &str) -> Vec<&str> {
    let starts: Vec<usize> = golden
        .match_indices("=== ")
        .map(|(start, _)| start)
        .filter(|&start| start == 0 || golden.as_bytes()[start - 1] == b'\n')
        .chain([golden.len()])
        .collect();
    starts.windows(2).map(|w| &golden[w[0]..w[1]]).collect()
}

#[test]
fn fixtures_parse_as_recorded() -> Result {
    let engine_state = test().engine_state;
    let root = WORKSPACE_ROOT.join(FIXTURES);
    let update = std::env::var_os(UPDATE_GOLDEN).is_some();

    let mut areas = vec![];
    for verdict in ["accept", "reject"] {
        let mut dirs: Vec<_> = std::fs::read_dir(root.join(verdict))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?;
        dirs.retain(|path| path.is_dir());
        dirs.sort();
        areas.extend(dirs);
    }

    let mut differences = String::new();
    for area in areas {
        let files = nu_files(&area);
        let actual: String = files
            .iter()
            .map(|path| {
                let name = path.strip_prefix(&root).expect("fixture is under the root");
                let name = name.components().map(|c| c.as_os_str().to_string_lossy());
                render(&engine_state, path, &name.collect::<Vec<_>>().join("/"))
            })
            .collect();
        let golden = area.with_extension("golden");
        if update {
            std::fs::write(&golden, &actual)?;
            continue;
        }
        let expected = std::fs::read_to_string(&golden).unwrap_or_default();
        if expected == actual {
            continue;
        }
        let expected = entries(&expected);
        let actual = entries(&actual);
        let header = |entry: &str| entry.lines().next().unwrap_or_default().to_string();
        for entry in &actual {
            match expected.iter().find(|e| header(e) == header(entry)) {
                Some(recorded) if recorded == entry => {}
                Some(recorded) => {
                    writeln!(differences, "{}", StrComparison::new(*recorded, *entry))
                        .expect("writing to a String");
                }
                None => {
                    writeln!(differences, "not recorded:\n{entry}").expect("writing to a String")
                }
            }
        }
        for entry in &expected {
            if !actual.iter().any(|a| header(a) == header(entry)) {
                writeln!(differences, "no longer a fixture:\n{entry}")
                    .expect("writing to a String");
            }
        }
    }
    assert!(
        differences.is_empty(),
        "the parse of these fixtures differs from their golden files (left: recorded, right: \
         now). If the change is intended, run `{UPDATE_GOLDEN}=1 cargo test --test tests -- \
         parsing::language` and review the diff of the golden files.\n{differences}"
    );
    Ok(())
}

/// Every `reject` snippet makes the parse or the compiler report an error.
#[test]
fn reject_fixtures_report_errors() -> Result {
    let engine_state = test().engine_state;
    let files = nu_files(WORKSPACE_ROOT.join(FIXTURES).join("reject"));
    let accepted: Vec<_> = files
        .iter()
        .filter(|path| {
            let source = std::fs::read(path).expect("fixture is readable");
            let (working_set, _) = parse_file(&engine_state, path, &source, true);
            working_set.parse_errors.is_empty() && working_set.compile_errors.is_empty()
        })
        .collect();
    assert!(
        accepted.is_empty(),
        "these rejected snippets parse without errors: {accepted:#?}"
    );
    Ok(())
}
