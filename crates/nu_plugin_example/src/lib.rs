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
            Box::new(Main),
            Box::new(One),
            Box::new(Two),
            Box::new(Three),
            Box::new(Config),
            Box::new(Env),
            Box::new(DisableGc),
        ]
    }
}
