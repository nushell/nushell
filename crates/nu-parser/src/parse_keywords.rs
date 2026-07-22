use crate::{lite_parser::LiteCommand, parser::parse_call};
use nu_protocol::{
    DeclId, ParseError, Span, Type,
    ast::{Expr, Expression, Pipeline},
    engine::StateWorkingSet,
};

/// These parser keywords can be aliased
pub const ALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[
    b"if",
    b"match",
    b"try",
    b"overlay",
    b"overlay hide",
    b"overlay new",
    b"overlay use",
];

/// These parser keywords cannot be aliased (either not possible, or support not yet added)
pub const UNALIASABLE_PARSER_KEYWORDS: &[&[u8]] = &[
    b"alias",
    b"const",
    b"def",
    b"extern",
    b"module",
    b"use",
    b"export",
    b"export alias",
    b"export const",
    b"export def",
    b"export extern",
    b"export module",
    b"export use",
    b"for",
    b"loop",
    b"while",
    b"return",
    b"break",
    b"continue",
    b"let",
    b"mut",
    b"hide",
    b"export-env",
    b"source-env",
    b"source",
    b"run",
    b"where",
    b"plugin use",
];

/// Check whether spans start with a parser keyword that can be aliased
pub fn is_unaliasable_parser_keyword(working_set: &StateWorkingSet, spans: &[Span]) -> bool {
    // try two words
    if let (Some(&span1), Some(&span2)) = (spans.first(), spans.get(1)) {
        let cmd_name = working_set.get_span_contents(Span::append(span1, span2));
        return UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name);
    }

    // try one word
    if let Some(&span1) = spans.first() {
        let cmd_name = working_set.get_span_contents(span1);
        UNALIASABLE_PARSER_KEYWORDS.contains(&cmd_name)
    } else {
        false
    }
}

/// Returns true if `name` matches any parser keyword (aliasable or unaliasable).
///
/// Used to prevent custom commands and aliases from shadowing language keywords
/// (e.g. `def def [] {}`), which can break subsequent parsing and previously
/// panicked in the REPL. This applies everywhere, including module exports —
/// `use mod *` would otherwise bring a bare keyword-named command into scope.
///
/// Also used when a module is named after a keyword and exports `main`: invoking
/// that entry point would use the module name as a bare command, which the parser
/// intercepts before command lookup.
///
/// Multi-word keywords such as `export def` and `overlay use` are included in the
/// lists; they will not match normal single-token definition or module names. See
/// [`single_word_parser_keywords`].
pub fn is_parser_keyword(name: &[u8]) -> bool {
    ALIASABLE_PARSER_KEYWORDS.contains(&name) || UNALIASABLE_PARSER_KEYWORDS.contains(&name)
}

/// Single-token parser keyword names that cannot be used as command or alias names.
///
/// Multi-word entries (`export def`, `overlay use`, …) are omitted because they
/// cannot appear as a single definition name token.
pub fn single_word_parser_keywords() -> impl Iterator<Item = &'static str> {
    ALIASABLE_PARSER_KEYWORDS
        .iter()
        .chain(UNALIASABLE_PARSER_KEYWORDS.iter())
        .filter_map(|bytes| {
            let name = std::str::from_utf8(bytes).ok()?;
            (!name.contains(' ')).then_some(name)
        })
}

/// If `name` is a parser keyword, records [`ParseError::NameIsKeyword`] and returns `true`.
///
/// `kind` is embedded in the error (e.g. `"command"` or `"alias"`). Callers should
/// abort the definition when this returns `true`.
pub fn reject_parser_keyword_name(
    working_set: &mut StateWorkingSet,
    name: &str,
    kind: &str,
    span: Span,
) -> bool {
    if is_parser_keyword(name.as_bytes()) {
        working_set.error(ParseError::NameIsKeyword(
            name.to_owned(),
            kind.to_owned(),
            span,
        ));
        true
    } else {
        false
    }
}

/// Find a keyword declaration by name, ignoring any non-keyword decls that may
/// have shadowed it in normal name lookup.
///
/// Prefer this when resolving parser-keyword commands such as `def`, `extern`,
/// or `run`, so a user-defined command of the same name cannot hijack parsing.
pub(crate) fn find_keyword_decl(working_set: &StateWorkingSet, name: &[u8]) -> Option<DeclId> {
    (0..working_set.num_decls())
        .map(DeclId::new)
        .find(|decl_id| {
            let decl = working_set.get_decl(*decl_id);
            decl.name().as_bytes() == name && decl.is_keyword()
        })
}

/// This is a new more compact method of calling parse_xxx() functions without repeating the
/// parse_call() in each function. Remaining keywords can be moved here.
pub fn parse_keyword(working_set: &mut StateWorkingSet, lite_command: &LiteCommand) -> Pipeline {
    let orig_parse_errors_len = working_set.parse_errors.len();

    let call_expr = parse_call(
        working_set,
        &lite_command.parts,
        lite_command.parts[0],
        None,
    );

    // If an error occurred, don't invoke the keyword-specific functionality
    if working_set.parse_errors.len() > orig_parse_errors_len {
        return Pipeline::from_vec(vec![call_expr]);
    }

    if let Expression {
        expr: Expr::Call(call),
        ..
    } = call_expr.clone()
    {
        // Apply parse keyword side effects
        let cmd = working_set.get_decl(call.decl_id);
        // check help flag first.
        if call.named_iter().any(|(flag, _, _)| flag.item == "help") {
            let call_span = call.span();
            return Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                call_span,
                Type::Any,
            )]);
        }

        match cmd.name() {
            "overlay hide" => crate::parse_module::parse_overlay_hide(working_set, call),
            "overlay new" => crate::parse_module::parse_overlay_new(working_set, call),
            "overlay use" => crate::parse_module::parse_overlay_use(working_set, call),
            #[cfg(feature = "plugin")]
            "plugin use" => crate::parse_source::parse_plugin_use(working_set, call),
            _ => Pipeline::from_vec(vec![call_expr]),
        }
    } else {
        Pipeline::from_vec(vec![call_expr])
    }
}

// Re-exports
pub use crate::parse_alias::parse_alias;
pub use crate::parse_bindings::{parse_const, parse_let, parse_mut};
pub use crate::parse_def::{
    parse_attribute_block, parse_def, parse_def_predecl, parse_extern, parse_for,
};
pub use crate::parse_module::{
    parse_export_env, parse_export_in_block, parse_export_in_module, parse_hide, parse_module,
    parse_module_block, parse_module_file_or_dir, parse_overlay_hide, parse_overlay_new,
    parse_overlay_use, parse_use,
};
pub use crate::parse_source::{
    LIB_DIRS_VAR, find_dirs_var, find_in_dirs, find_main_block_id_in_script, parse_run,
    parse_run_expr, parse_source, parse_where, parse_where_expr,
};
#[cfg(feature = "plugin")]
pub use crate::parse_source::{PLUGIN_DIRS_VAR, parse_plugin_use};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_parser_keyword_matches_single_word_keywords() {
        assert!(is_parser_keyword(b"def"));
        assert!(is_parser_keyword(b"let"));
        assert!(is_parser_keyword(b"if")); // aliasable
        assert!(is_parser_keyword(b"overlay")); // aliasable
        assert!(is_parser_keyword(b"where"));
    }

    #[test]
    fn is_parser_keyword_rejects_ordinary_command_names() {
        assert!(!is_parser_keyword(b"ls"));
        assert!(!is_parser_keyword(b"my-command"));
        assert!(!is_parser_keyword(b""));
    }

    #[test]
    fn is_parser_keyword_includes_multi_word_entries() {
        // Present in the tables; normal `def`/`alias` names are single tokens so
        // these only matter for completeness of the keyword set.
        assert!(is_parser_keyword(b"export def"));
        assert!(is_parser_keyword(b"overlay use"));
    }

    #[test]
    fn single_word_parser_keywords_excludes_multi_word_and_matches_is_parser_keyword() {
        let names: Vec<_> = single_word_parser_keywords().collect();
        assert!(names.contains(&"def"));
        assert!(names.contains(&"if"));
        assert!(!names.iter().any(|n| n.contains(' ')));
        for name in &names {
            assert!(
                is_parser_keyword(name.as_bytes()),
                "{name} should be a parser keyword"
            );
        }
    }
}
