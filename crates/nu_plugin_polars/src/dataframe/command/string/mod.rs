mod concat_str;
mod contains;
mod str_join;
mod str_lengths;
mod str_replace;
mod str_replace_all;
mod str_slice;
mod str_split;
mod str_strip_chars;
mod to_lowercase;
mod to_uppercase;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use concat_str::ExprConcatStr;
pub use contains::Contains;
pub use str_join::StrJoin;
pub use str_lengths::StrLengths;
pub use str_replace::StrReplace;
pub use str_replace_all::StrReplaceAll;
pub use str_slice::StrSlice;
pub use to_lowercase::ToLowerCase;
pub use to_uppercase::ToUpperCase;

pub(crate) fn string_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ExprConcatStr),
        Box::new(Contains),
        Box::new(StrReplace),
        Box::new(StrReplaceAll),
        Box::new(str_split::StrSplit),
        Box::new(str_strip_chars::StrStripChars),
        Box::new(StrJoin),
        Box::new(StrLengths),
        Box::new(StrSlice),
        Box::new(ToLowerCase),
        Box::new(ToUpperCase),
    ]
}
