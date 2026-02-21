use nu_protocol::{
    Value,
    engine::{EngineState, Stack},
};
// Should read config from $env.config.clip instead of $env.config.plugins.clip, but we want to avoid breaking existing user configs.
pub(crate) fn get_clip_config(engine_state: &EngineState, stack: &mut Stack) -> Option<Value> {
    let config = stack.get_config(engine_state);
    config
        .plugins
        .get("clip")
        .or_else(|| config.plugins.get("clipboard"))
        .or_else(|| config.plugins.get("nu_plugin_clipboard"))
        .cloned()
}
