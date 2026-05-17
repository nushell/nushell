mod math;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

use math::ExprMath;

pub(crate) fn computation_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![Box::new(ExprMath)]
}
