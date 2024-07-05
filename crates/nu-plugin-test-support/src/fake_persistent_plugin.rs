use std::{
    any::Any,
    sync::{Arc, OnceLock},
};

use nu_plugin_engine::{GetPlugin, PluginInterface};
use nu_protocol::{
    engine::{EngineState, Stack},
    PluginGcConfig, PluginIdentity, PluginMetadata, RegisteredPlugin, ShellError,
};

pub struct FakePersistentPlugin {
    identity: PluginIdentity,
    plugin: OnceLock<PluginInterface>,
}

impl FakePersistentPlugin {
    pub fn new(identity: PluginIdentity) -> FakePersistentPlugin {
        FakePersistentPlugin {
            identity,
            plugin: OnceLock::new(),
        }
    }

    pub fn initialize(&self, interface: PluginInterface) {
        self.plugin.set(interface).unwrap_or_else(|_| {
            panic!("Tried to initialize an already initialized FakePersistentPlugin");
        })
    }
}

impl RegisteredPlugin for FakePersistentPlugin {
    fn identity(&self) -> &PluginIdentity {
        &self.identity
    }

    fn is_running(&self) -> bool {
        true
    }

    fn pid(&self) -> Option<u32> {
        None
    }

    fn metadata(&self) -> Option<PluginMetadata> {
        None
    }

    fn set_metadata(&self, _metadata: Option<PluginMetadata>) {}

    fn set_gc_config(&self, _gc_config: &PluginGcConfig) {
        // We don't have a GC
    }

    fn stop(&self) -> Result<(), ShellError> {
        // We can't stop
        Ok(())
    }

    fn reset(&self) -> Result<(), ShellError> {
        // We can't stop
        Ok(())
    }

    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

impl GetPlugin for FakePersistentPlugin {
    fn get_plugin(
        self: Arc<Self>,
        _context: Option<(&EngineState, &mut Stack)>,
    ) -> Result<PluginInterface, ShellError> {
        self.plugin
            .get()
            .cloned()
            .ok_or_else(|| ShellError::PluginFailedToLoad {
                msg: "FakePersistentPlugin was not initialized".into(),
            })
    }
}
