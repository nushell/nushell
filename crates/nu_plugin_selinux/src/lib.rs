mod commands;
pub use commands::*;
use nu_command::Ls;
use nu_plugin::{Plugin, PluginCommand};
pub struct SELinuxPlugin;

impl Plugin for SELinuxPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(SELinuxLs { ls: Ls })]
    }
}
