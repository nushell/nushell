mod selector_all;
mod selector_by_dtype;
mod selector_by_name;
mod selector_first;
mod selector_last;
mod selector_matches;
mod selector_not;
mod selector_starts_with;
mod selector_stub;

use nu_plugin::PluginCommand;

use crate::PolarsPlugin;

pub(crate) fn selector_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(selector_stub::SelectorCmd),
        Box::new(selector_all::SelectorAll),
        Box::new(selector_by_dtype::SelectorByDtype),
        Box::new(selector_by_name::SelectorByName),
        Box::new(selector_first::SelectorFirst),
        Box::new(selector_last::SelectorLast),
        Box::new(selector_matches::SelectorMatches),
        Box::new(selector_not::SelectorNot),
        Box::new(selector_starts_with::SelectorStartsWith),
    ]
}
