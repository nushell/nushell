mod math;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub(crate) fn computation_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    math::commands()
}
