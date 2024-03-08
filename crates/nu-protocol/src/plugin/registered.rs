use std::{any::Any, sync::Arc};

use crate::{PluginGcConfig, PluginIdentity, ShellError};

/// Trait for plugins registered in the [`EngineState`](crate::EngineState).
pub trait RegisteredPlugin: Send + Sync {
    /// The identity of the plugin - its filename, shell, and friendly name.
    fn identity(&self) -> &PluginIdentity;

    /// True if the plugin is currently running.
    fn is_running(&self) -> bool;

    /// Process ID of the plugin executable, if running.
    fn pid(&self) -> Option<u32>;

    /// Set garbage collection config for the plugin.
    fn set_gc_config(&self, gc_config: &PluginGcConfig);

    /// Stop the plugin.
    fn stop(&self) -> Result<(), ShellError>;

    /// Cast the pointer to an [`Any`] so that its concrete type can be retrieved.
    ///
    /// This is necessary in order to allow `nu_plugin` to handle the implementation details of
    /// plugins.
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}
