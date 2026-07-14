use crate::{lite_parser::LiteCommand, parser::parse_call};
use nu_protocol::{
    Span, Type,
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
