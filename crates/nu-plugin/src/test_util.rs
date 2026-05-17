use nu_plugin_core::interface_test_util::TestCase;
use nu_plugin_protocol::{PluginInput, PluginOutput};

use crate::plugin::EngineInterfaceManager;

pub trait TestCaseExt {
    /// Create a new [`EngineInterfaceManager`] that writes to this test case.
    fn engine(&self) -> EngineInterfaceManager;
}

impl TestCaseExt for TestCase<PluginInput, PluginOutput> {
    fn engine(&self) -> EngineInterfaceManager {
        EngineInterfaceManager::new(self.clone())
    }
}
