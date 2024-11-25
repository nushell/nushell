use commands::{Http, HttpDelete, HttpGet, HttpHead, HttpOptions, HttpPatch, HttpPost, HttpPut};
use nu_plugin::{Plugin, PluginCommand};

mod commands;

#[derive(Default)]
pub struct HttpPlugin;

impl Plugin for HttpPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(HttpDelete),
            Box::new(HttpGet),
            Box::new(HttpHead),
            Box::new(Http),
            Box::new(HttpOptions),
            Box::new(HttpPatch),
            Box::new(HttpPost),
            Box::new(HttpPut),
        ]
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::ShellError;

    pub fn examples(command: &impl PluginCommand) -> Result<(), ShellError> {
        let mut plugin_test = PluginTest::new(command.name(), HttpPlugin.into())?;
        plugin_test.test_examples(&command.examples())
    }
}
