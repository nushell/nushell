use std::{ops::Deref, sync::Arc};

use nu_plugin::{create_plugin_signature, Plugin, PluginDeclaration};
use nu_protocol::{engine::StateWorkingSet, RegisteredPlugin, ShellError};

use crate::{fake_persistent_plugin::FakePersistentPlugin, spawn_fake_plugin::spawn_fake_plugin};

/// Register all of the commands from the plugin into the [`StateWorkingSet`]
pub fn fake_register(
    working_set: &mut StateWorkingSet,
    name: &str,
    plugin: Arc<impl Plugin + Send + 'static>,
) -> Result<Arc<FakePersistentPlugin>, ShellError> {
    let reg_plugin = spawn_fake_plugin(name, plugin.clone())?;
    let reg_plugin_clone = reg_plugin.clone();

    for command in plugin.commands() {
        let signature = create_plugin_signature(command.deref());
        let decl = PluginDeclaration::new(reg_plugin.clone(), signature);
        working_set.add_decl(Box::new(decl));
    }

    let identity = reg_plugin.identity().clone();
    working_set.find_or_create_plugin(&identity, move || reg_plugin);

    Ok(reg_plugin_clone)
}
