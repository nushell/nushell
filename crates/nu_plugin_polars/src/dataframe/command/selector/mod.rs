mod selector_all;
mod selector_alpha;
mod selector_alphanumeric;
mod selector_array;
mod selector_binary;
mod selector_boolean;
mod selector_by_dtype;
mod selector_by_index;
mod selector_by_name;
mod selector_categorical;
mod selector_contains;
mod selector_date;
mod selector_datetime;
mod selector_decimal;
mod selector_digit;
mod selector_duration;
mod selector_empty;
mod selector_ends_with;
mod selector_exclude;
mod selector_first;
mod selector_float;
mod selector_integer;
mod selector_last;
mod selector_list;
mod selector_matches;
mod selector_nested;
mod selector_not;
mod selector_numeric;
mod selector_object;
mod selector_polars_enum;
mod selector_polars_struct;
mod selector_signed_integer;
mod selector_starts_with;
mod selector_string;
mod selector_stub;
mod selector_temporal;
mod selector_unsigned_integer;

use nu_plugin::PluginCommand;

use crate::PolarsPlugin;

pub(crate) fn selector_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(selector_all::SelectorAll),
        Box::new(selector_alpha::SelectorAlpha),
        Box::new(selector_alphanumeric::SelectorAlphanumeric),
        Box::new(selector_array::SelectorArray),
        Box::new(selector_binary::SelectorBinary),
        Box::new(selector_boolean::SelectorBoolean),
        Box::new(selector_by_dtype::SelectorByDtype),
        Box::new(selector_by_index::SelectorByIndex),
        Box::new(selector_by_name::SelectorByName),
        Box::new(selector_categorical::SelectorCategorical),
        Box::new(selector_contains::SelectorContains),
        Box::new(selector_date::SelectorDate),
        Box::new(selector_datetime::SelectorDatetime),
        Box::new(selector_decimal::SelectorDecimal),
        Box::new(selector_digit::SelectorDigit),
        Box::new(selector_duration::SelectorDuration),
        Box::new(selector_empty::SelectorEmpty),
        Box::new(selector_ends_with::SelectorEndsWith),
        Box::new(selector_exclude::SelectorExclude),
        Box::new(selector_first::SelectorFirst),
        Box::new(selector_float::SelectorFloat),
        Box::new(selector_integer::SelectorInteger),
        Box::new(selector_last::SelectorLast),
        Box::new(selector_list::SelectorList),
        Box::new(selector_matches::SelectorMatches),
        Box::new(selector_nested::SelectorNested),
        Box::new(selector_not::SelectorNot),
        Box::new(selector_numeric::SelectorNumeric),
        Box::new(selector_object::SelectorObject),
        Box::new(selector_polars_enum::SelectorPolarsEnum),
        Box::new(selector_polars_struct::SelectorPolarsStruct),
        Box::new(selector_signed_integer::SelectorSignedInteger),
        Box::new(selector_starts_with::SelectorStartsWith),
        Box::new(selector_string::SelectorString),
        Box::new(selector_stub::SelectorCmd),
        Box::new(selector_temporal::SelectorTemporal),
        Box::new(selector_unsigned_integer::SelectorUnsignedInteger),
    ]
}
