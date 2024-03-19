use nu_plugin::{Plugin, PluginCommand};

mod commands;
mod example;

pub use commands::*;
pub use example::Example;

impl Plugin for Example {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        // This is a list of all of the commands you would like Nu to register when your plugin is
        // loaded.
        //
        // If it doesn't appear on this list, it won't be added.
        vec![
            Box::new(NuExample1),
            Box::new(NuExample2),
            Box::new(NuExample3),
            Box::new(NuExampleConfig),
            Box::new(NuExampleEnv),
            Box::new(NuExampleDisableGc),
        ]
    }
}
