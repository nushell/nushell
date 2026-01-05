mod selector_all;
mod selector_by_dtype;
mod selector_by_name;
mod selector_first;
mod selector_last;

use nu_plugin::PluginCommand;

use crate::PolarsPlugin;

pub use selector_all::SelectorAll;
pub use selector_by_dtype::SelectorByDtype;
pub use selector_by_name::SelectorByName;
pub use selector_first::SelectorFirst;
pub use selector_last::SelectorLast;

pub(crate) fn selector_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(SelectorAll),
        Box::new(SelectorByDtype),
        Box::new(SelectorByName),
        Box::new(SelectorFirst),
        Box::new(SelectorLast),
    ]
}
