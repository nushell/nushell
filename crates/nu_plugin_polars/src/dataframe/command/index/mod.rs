mod arg_max;
mod arg_min;
mod arg_sort;
mod arg_unique;
mod set_with_idx;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use arg_max::ArgMax;
pub use arg_min::ArgMin;
pub use arg_sort::ArgSort;
pub use arg_unique::ArgUnique;
pub use set_with_idx::SetWithIndex;

pub(crate) fn index_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ArgMax),
        Box::new(ArgMin),
        Box::new(ArgSort),
        Box::new(ArgUnique),
        Box::new(SetWithIndex),
    ]
}
