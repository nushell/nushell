mod concat_str;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use crate::dataframe::expressions::concat_str::ExprConcatStr;

pub(crate) fn expr_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![Box::new(ExprConcatStr)]
}
