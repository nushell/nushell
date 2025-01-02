mod concat_str;
mod contains;
mod replace;
mod replace_all;
mod str_join;
mod str_lengths;
mod str_slice;
mod str_split;
mod to_lowercase;
mod to_uppercase;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use concat_str::ExprConcatStr;
pub use contains::Contains;
pub use replace::Replace;
pub use replace_all::ReplaceAll;
pub use str_join::StrJoin;
pub use str_lengths::StrLengths;
pub use str_slice::StrSlice;
pub use to_lowercase::ToLowerCase;
pub use to_uppercase::ToUpperCase;

pub(crate) fn string_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ExprConcatStr),
        Box::new(Contains),
        Box::new(Replace),
        Box::new(ReplaceAll),
        Box::new(str_split::StrSplit),
        Box::new(StrJoin),
        Box::new(StrLengths),
        Box::new(StrSlice),
        Box::new(ToLowerCase),
        Box::new(ToUpperCase),
    ]
}
