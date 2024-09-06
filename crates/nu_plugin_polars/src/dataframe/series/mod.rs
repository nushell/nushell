mod cumulative;
mod n_null;
mod n_unique;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use cumulative::Cumulative;
pub use n_null::NNull;
pub use n_unique::NUnique;

pub(crate) fn series_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![Box::new(Cumulative), Box::new(NNull), Box::new(NUnique)]
}
