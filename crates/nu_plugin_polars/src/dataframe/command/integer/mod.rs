mod to_decimal;
mod to_integer;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use to_decimal::ToDecimal;
pub use to_integer::ToInteger;

pub(crate) fn integer_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![Box::new(ToDecimal), Box::new(ToInteger)]
}
