//! The winnow front end parses and lexes with its own code: of the rest of nu-parser it uses
//! only what reads no source text. That is the code both front ends share once a statement is
//! parsed: the `finish_*` functions that declare, load and check, compiling and capture
//! discovery, type checking, name lookup and the checks on a name already read, and plain data
//! (`SHARED`).
//!
//! The test reads `src/winnow`'s sources and collects every path it names under `crate::`
//! outside `crate::winnow` (in `use` statements and inline). Each must be in `SHARED`: a new
//! one that reads no source text is added there, and nu-parser's parsing or lexing gets a
//! winnow implementation instead. One no longer used is struck from it.
#![allow(clippy::unwrap_used, reason = "tests may panic on failure")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// What reads no source text: shared with the classic front end for good.
const SHARED: &[&str] = &[
    // The tokens nu-parser's `parse_block` hands to `winnow::nested_block` (an entry point).
    "Token",
    "TokenContents",
    "exportable::Exportable",
    // Checks on a name already read, which both front ends must apply alike.
    "parse_alias::alias_name",
    "parse_alias::alias_name_fits_module",
    "parse_def::reject_command_name",
    "parse_signatures::ensure_not_reserved_variable_name",
    // Declaring, loading and checking a parsed statement.
    "parse_alias::finish_alias",
    "parse_def::DefCall",
    "parse_def::finish_def",
    "parse_def::finish_extern",
    "parse_def::finish_for",
    "parse_def::has_flag_const",
    "parse_def::rest_param_is_type_annotated",
    "parse_def::yielded_type",
    "parse_module::add_exportables",
    "parse_module::double_main_span",
    "parse_module::exported_module",
    "parse_module::finish_export_env",
    "parse_module::finish_hide",
    "parse_module::finish_module",
    "parse_module::finish_use",
    "parse_module::parse_overlay_hide",
    "parse_module::parse_overlay_new",
    "parse_module::parse_overlay_use",
    "parse_module::warp_export_call",
    "parse_signatures::Arg",
    "parse_signatures::assemble_signature",
    "parse_source::LIB_DIRS_VAR",
    "parse_source::find_dirs_var",
    "parse_source::finish_run",
    "parse_source::finish_source",
    "parse_source::parse_plugin_use",
    "parse_type_alias::finish_type_alias",
    // Calls: their kind, their check against the signature, and shapes' properties.
    "parse_calls::CallKind",
    "parse_calls::accepts_any_bareword",
    "parse_calls::check_call",
    "parse_calls::shape_allows_negative_number",
    "parse_helpers::PERCENT_FORCED_BUILTIN_PARSER_INFO",
    "parse_helpers::SPREAD_OPERATOR_STR",
    "parse_helpers::extract_spread_list",
    "parse_helpers::extract_spread_record",
    "parse_helpers::garbage",
    // Name lookup.
    "parse_calls::find_decl_with_command_type",
    "parse_calls::find_longest_decl",
    "parse_calls::find_longest_decl_with_prefix",
    "parse_keywords::find_keyword_decl",
    "parse_keywords::is_parser_keyword",
    // Blocks, compiling and types.
    "parse_captures_compile::compile_block",
    "parse_captures_compile::compile_block_with_id",
    "parse_captures_compile::wrap_element_with_collect",
    "parse_expressions::check_record_key_or_value",
    "parse_expressions::table_type",
    "parse_pipelines::finish_block",
    "parse_pipelines::hide_unlet_constants",
    "type_check::check_pipeline_type",
    "type_check::check_range_types",
    "type_check::math_result_type",
    "type_check::negation_type",
    "type_check::type_compatible",
];

/// Every path under `crate::` outside `crate::winnow` that `src/winnow` names, with the files
/// naming it.
fn nu_parser_paths() -> BTreeMap<String, BTreeSet<String>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/winnow");
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    let mut paths: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let name = file.strip_prefix(&root).unwrap().display().to_string();
        let source = std::fs::read_to_string(&file).unwrap();
        let mut code = without_comments(&source);
        // `use crate::...;` statements, then blanked out so the inline scan skips them.
        while let Some(start) = code.find("use crate::") {
            let end = start + code[start..].find(';').unwrap();
            let tree = &code[start + "use crate::".len()..end];
            for path in flatten_use_tree(tree) {
                paths.entry(path).or_default().insert(name.clone());
            }
            code.replace_range(start..end, &" ".repeat(end - start));
        }
        let mut rest = code.as_str();
        while let Some(at) = rest.find("crate::") {
            rest = &rest[at + "crate::".len()..];
            let len = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
                .unwrap_or(rest.len());
            let path = rest[..len].trim_end_matches(':');
            // An inline path names an item, or a module followed by one.
            let path = path.split("::").take(2).collect::<Vec<_>>().join("::");
            paths.entry(path).or_default().insert(name.clone());
        }
    }
    paths.retain(|path, _| path != "winnow" && !path.starts_with("winnow::"));
    paths
}

fn collect_rust_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// `source` without its line comments, documentation included (an intra-doc link names a
/// path it does not use).
fn without_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.trim_start().starts_with("//") {
            true => "",
            false => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The full paths of a `use` tree (`a::{b, c::{d as e}}` gives `a::b` and `a::c::d`).
fn flatten_use_tree(tree: &str) -> Vec<String> {
    let tree = tree.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    flatten_into(&tree, "", &mut out);
    out
}

fn flatten_into(tree: &str, prefix: &str, out: &mut Vec<String>) {
    let mut depth = 0;
    let mut start = 0;
    let mut items = Vec::new();
    for (index, c) in tree.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(&tree[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    items.push(&tree[start..]);
    for item in items
        .into_iter()
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let joined = |path: &str| match prefix.is_empty() {
            true => path.to_string(),
            false => format!("{prefix}::{path}"),
        };
        if item.starts_with('{') && item.ends_with('}') {
            flatten_into(&item[1..item.len() - 1], prefix, out);
        } else if let Some(open) = item.find("::{")
            && item.ends_with('}')
        {
            flatten_into(&item[open + 3..item.len() - 1], &joined(&item[..open]), out);
        } else {
            // A rename (`parse_value as classic_value`) names the item before it.
            out.push(joined(item.split(" as ").next().unwrap_or(item)));
        }
    }
}

#[test]
fn winnow_front_end_uses_only_shared_nu_parser_code() {
    let used = nu_parser_paths();
    let unclassified: Vec<_> = used
        .iter()
        .filter(|(path, _)| !SHARED.contains(&path.as_str()))
        .collect();
    assert!(
        unclassified.is_empty(),
        "nu-parser items src/winnow uses that are not in SHARED (one that reads no source \
         text goes there; parsing or lexing gets a winnow implementation): {unclassified:?}"
    );
    let unused: Vec<_> = SHARED
        .iter()
        .filter(|path| !used.contains_key(**path))
        .collect();
    assert!(
        unused.is_empty(),
        "no longer used by src/winnow: strike from SHARED: {unused:?}"
    );
}
