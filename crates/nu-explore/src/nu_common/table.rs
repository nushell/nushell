use crate::nu_common::NuConfig;
use nu_color_config::StyleComputer;
use nu_protocol::{Record, Signals, Value};
use nu_table::{
    ExpandedTable, TableOpts,
    common::{nu_value_to_string, nu_value_to_string_clean},
};

pub fn try_build_table(
    value: Value,
    signals: &Signals,
    config: &NuConfig,
    style_computer: StyleComputer,
) -> String {
    let span = value.span();
    let opts = TableOpts::new(
        config,
        style_computer,
        signals,
        span,
        usize::MAX,
        config.table.mode,
        0,
        false,
    );
    match value {
        Value::List { vals, .. } => try_build_list(vals, opts),
        Value::Record { val, .. } => try_build_map(&val, opts),
        val @ Value::String { .. } => {
            nu_value_to_string_clean(&val, config, &opts.style_computer).0
        }
        val => nu_value_to_string(&val, config, &opts.style_computer).0,
    }
}

fn try_build_map(record: &Record, opts: TableOpts<'_>) -> String {
    let result = ExpandedTable::new(None, false, String::new()).build_map(record, opts.clone());
    match result {
        Ok(Some(result)) => result,
        _ => {
            let value = Value::record(record.clone(), opts.span);
            nu_value_to_string(&value, opts.config, &opts.style_computer).0
        }
    }
}

fn try_build_list(vals: Vec<Value>, opts: TableOpts<'_>) -> String {
    let result = ExpandedTable::new(None, false, String::new()).build_list(&vals, opts.clone());
    match result {
        Ok(Some(out)) => out,
        _ => {
            // it means that the list is empty
            let value = Value::list(vals, opts.span);
            nu_value_to_string(&value, opts.config, &opts.style_computer).0
        }
    }
}
