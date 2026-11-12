//! `open`, `save` and `http` pick a `from <ext>` / `to <ext>` converter by name and run it, but an
//! alias can't be run on its own. When the converter found is an alias, they report it with
//! [`alias_converter_error`] instead of failing with an internal error (#18725).

use nu_protocol::{
    Alias, ShellError, Span, ast::Expr, engine::EngineState, shell_error::generic::GenericError,
};

/// The error for an alias found as the converter. `command` is what failed (`open`, `save`,
/// `http`), `what` the action in the title (`open file`), and `span` the argument to label.
/// `piped` is the same conversion written out as a pipeline, with `{}` where the alias's expansion
/// goes: `open --raw x.json | {}`.
///
/// The help suggests a `def` of the same name, which those commands can run, unless the alias
/// wraps the very command it shadows (`alias "from json" = from json --objects`): inside such a
/// `def` the name would call the `def` itself, so the help shows the explicit pipeline instead.
pub(crate) fn alias_converter_error(
    engine_state: &EngineState,
    alias: &Alias,
    command: &str,
    what: &str,
    piped: &str,
    span: Span,
) -> ShellError {
    let expansion =
        String::from_utf8_lossy(engine_state.get_span_contents(alias.wrapped_call.span));
    let expansion = expansion.trim();
    let shadows = matches!(&alias.wrapped_call.expr, Expr::Call(call) if engine_state.get_decl(call.decl_id).name() == alias.name);
    let help = if shadows {
        format!(
            "`{command}` can only use a command as a converter. Convert explicitly instead: `{}`",
            piped.replace("{}", expansion)
        )
    } else {
        format!(
            "`{command}` can only use a command as a converter. Define one with `def` instead: `def '{}' [] {{ {expansion} }}`",
            alias.name
        )
    };
    ShellError::Generic(
        GenericError::new(
            format!("Can't {what} with alias `{}`", alias.name),
            format!("`{}` is an alias, not a command", alias.name),
            span,
        )
        .with_help(help),
    )
}
