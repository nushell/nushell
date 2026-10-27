//! What the harness binaries share: the engine state nu-parser runs in.

use nu_protocol::{
    Span, Type, Value,
    engine::{EngineState, StateWorkingSet},
};

/// The engine state the `nu` binary parses with: the commands of nushell's
/// `src/command_context.rs` (`add_command_context`), added in the same order
/// and with the `plugin` feature on, then the `$nu` and `$NU_LIB_DIRS` constants
/// and, with `std`, the standard library and its prelude.
///
/// Every command the `nu` binary knows is known here, so a name nu-parser
/// cannot resolve is an external command in both (`print` comes from
/// `nu-cli`, `plugin use` from `nu-cmd-plugin`). No plugin is registered,
/// as in `nu -n`.
///
/// Every experimental option is first set to its default, whatever
/// `NU_EXPERIMENTAL_OPTIONS` says: `ExperimentalOption::get` falls back to that
/// variable for an option never set, so an exported `winnow-parser` (or `all`)
/// would make the reference nu-parser parse with the winnow parser, and `all` would
/// add the commands of an option such as `native-clip` to the engine.
pub fn engine(std: bool) -> EngineState {
    for option in nu_experimental::ALL {
        // The default `ExperimentalOption::get` gives an option never set.
        let default = matches!(option.status(), nu_experimental::Status::OptOut);
        // SAFETY: set once at startup, before the engine exists and anything is parsed.
        unsafe { option.set(default) };
    }
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_cmd_plugin::add_plugin_command_context(engine_state);
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    let engine_state = nu_cli::add_cli_context(engine_state);
    let engine_state = nu_explore::add_explore_context(engine_state);
    let mut engine_state = nu_tui::add_tui_context(engine_state);
    // `$nu` must exist before the standard library is parsed, as in the `nu` binary.
    engine_state.generate_nu_constant();
    add_lib_dirs_constant(&mut engine_state);
    if std && let Err(e) = nu_std::load_standard_library(&mut engine_state) {
        eprintln!("warning: could not load the standard library: {e}");
    }
    engine_state
}

/// `$NU_LIB_DIRS`, which the `nu` binary defines at startup and `parse_internal_call` hands to
/// `use`, `overlay use`, `source-env` and `nu-check`. It is empty, so that no module is found
/// outside the files parsed.
fn add_lib_dirs_constant(engine_state: &mut EngineState) {
    let mut working_set = StateWorkingSet::new(engine_state);
    let var_id = working_set.add_variable(
        b"$NU_LIB_DIRS".into(),
        Span::unknown(),
        Type::List(Box::new(Type::String)),
        false,
    );
    working_set.set_variable_const_val(var_id, Value::list(vec![], Span::unknown()));
    if let Err(e) = engine_state.merge_delta(working_set.render()) {
        eprintln!("warning: could not define $NU_LIB_DIRS: {e}");
    }
}
