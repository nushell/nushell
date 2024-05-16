use nu_plugin::{Plugin, PluginCommand};
use nu_protocol::PluginMetadata;

mod commands;
mod example;

pub use commands::*;
pub use example::ExamplePlugin;

impl Plugin for ExamplePlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        // This is a list of all of the commands you would like Nu to register when your plugin is
        // loaded.
        //
        // If it doesn't appear on this list, it won't be added.
        vec![
            Box::new(Main),
            // Basic demos
            Box::new(One),
            Box::new(Two),
            Box::new(Three),
            // Engine interface demos
            Box::new(Config),
            Box::new(Env),
            Box::new(ViewSpan),
            Box::new(DisableGc),
            // Stream demos
            Box::new(CollectBytes),
            Box::new(Echo),
            Box::new(ForEach),
            Box::new(Generate),
            Box::new(Seq),
            Box::new(Sum),
        ]
    }

    fn metadata(&self) -> PluginMetadata {
        // This reports the plugin's version to Nu, and we recommend that you include it.
        PluginMetadata::new().with_version(env!("CARGO_PKG_VERSION"))
    }
}
