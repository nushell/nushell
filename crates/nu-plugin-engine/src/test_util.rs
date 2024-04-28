use std::sync::Arc;

use nu_plugin_core::interface_test_util::TestCase;
use nu_plugin_protocol::{test_util::test_plugin_custom_value, PluginInput, PluginOutput};

use crate::{PluginCustomValueWithSource, PluginInterfaceManager, PluginSource};

pub trait TestCaseExt {
    /// Create a new [`PluginInterfaceManager`] that writes to this test case.
    fn plugin(&self, name: &str) -> PluginInterfaceManager;
}

impl TestCaseExt for TestCase<PluginOutput, PluginInput> {
    fn plugin(&self, name: &str) -> PluginInterfaceManager {
        PluginInterfaceManager::new(PluginSource::new_fake(name).into(), None, self.clone())
    }
}

pub fn test_plugin_custom_value_with_source() -> PluginCustomValueWithSource {
    PluginCustomValueWithSource::new(
        test_plugin_custom_value(),
        Arc::new(PluginSource::new_fake("test")),
    )
}
