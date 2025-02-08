use nu_protocol::engine::EngineState;

pub(crate) fn get_engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    #[cfg(feature = "plugin")]
    let engine_state = nu_cmd_plugin::add_plugin_command_context(engine_state);
    let engine_state = nu_command::add_shell_command_context(engine_state);
    let engine_state = nu_cmd_extra::add_extra_command_context(engine_state);
    let engine_state = nu_cli::add_cli_context(engine_state);
    nu_explore::add_explore_context(engine_state)
}
