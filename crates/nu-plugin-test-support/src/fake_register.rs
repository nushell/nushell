use std::sync::Arc;

use nu_plugin::{Plugin, PluginDeclaration};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    RegisteredPlugin, ShellError,
};

use crate::spawn_fake_plugin::spawn_fake_plugin;

/// Register all of the commands from the plugin into the [`StateWorkingSet`]
pub fn fake_register(
    working_set: &mut StateWorkingSet,
    name: &str,
    plugin: Arc<impl Plugin + Send + 'static>,
) -> Result<(), ShellError> {
    let reg_plugin = spawn_fake_plugin(name, plugin.clone())?;

    for command in plugin.commands() {
        let signature = command.signature();
        let decl = PluginDeclaration::new(reg_plugin.clone(), signature);
        working_set.add_decl(Box::new(decl));
    }

    let identity = reg_plugin.identity().clone();
    working_set.find_or_create_plugin(&identity, move || reg_plugin);

    Ok(())
}

/// Create an [`EngineState`] with the plugin's commands in it.
pub fn create_engine_state(
    plugin_name: &str,
    plugin: Arc<impl Plugin + Send + 'static>,
) -> Result<EngineState, ShellError> {
    let mut engine_state = EngineState::new();
    let mut working_set = StateWorkingSet::new(&engine_state);

    fake_register(&mut working_set, plugin_name, plugin)?;

    engine_state.merge_delta(working_set.render())?;
    Ok(engine_state)
}
