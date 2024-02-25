use std::sync::{atomic::AtomicBool, Arc};

use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
};

/// Object safe trait for abstracting operations required of the plugin context.
pub(crate) trait PluginExecutionContext: Send + Sync {
    /// The interrupt signal, if present
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>>;
}

/// The execution context of a plugin command. May be extended with more fields in the future.
pub(crate) struct PluginExecutionCommandContext {
    ctrlc: Option<Arc<AtomicBool>>,
}

impl PluginExecutionCommandContext {
    pub fn new(
        engine_state: &EngineState,
        _stack: &Stack,
        _call: &Call,
    ) -> PluginExecutionCommandContext {
        PluginExecutionCommandContext {
            ctrlc: engine_state.ctrlc.clone(),
        }
    }
}

impl PluginExecutionContext for PluginExecutionCommandContext {
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        self.ctrlc.as_ref()
    }
}

/// A bogus execution context for testing that doesn't really implement anything properly
#[cfg(test)]
pub(crate) struct PluginExecutionBogusContext;

#[cfg(test)]
impl PluginExecutionContext for PluginExecutionBogusContext {
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        None
    }
}
