//! What the harness binaries share: the engine state nu-parser runs in.

use nu_protocol::engine::EngineState;

/// The engine state the `nu` binary parses with: the commands of nushell's
/// `src/command_context.rs` (`add_command_context`), added in the same order
/// and with the `plugin` feature on, then the `$nu` constant and, with `std`,
/// the standard library and its prelude.
///
/// Every command the `nu` binary knows is known here, so a name nu-parser
/// cannot resolve is an external command in both (`print` comes from
/// `nu-cli`, `plugin use` from `nu-cmd-plugin`). No plugin is registered,
/// as in `nu -n`.
///
/// Every experimental option is first set to its default, whatever
/// `NU_EXPERIMENTAL_OPTIONS` says: `ExperimentalOption::get` falls back to that
/// variable for an option never set, so an exported `winnow-parser` (or `all`)
/// would make the reference nu-parser parse with this crate, and `all` would
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
    if std && let Err(e) = nu_std::load_standard_library(&mut engine_state) {
        eprintln!("warning: could not load the standard library: {e}");
    }
    engine_state
}
