use crate::nu_common::NuConfig;
use nu_color_config::StyleComputer;
use nu_protocol::{Record, Signals, Span, Value};
use nu_table::{
    common::{nu_value_to_string, nu_value_to_string_clean},
    ExpandedTable, TableOpts,
};

pub fn try_build_table(
    signals: &Signals,
    config: &NuConfig,
    style_computer: &StyleComputer,
    value: Value,
) -> String {
    let span = value.span();
    match value {
        Value::List { vals, .. } => try_build_list(vals, signals, config, span, style_computer),
        Value::Record { val, .. } => try_build_map(&val, span, style_computer, signals, config),
        val if matches!(val, Value::String { .. }) => {
            nu_value_to_string_clean(&val, config, style_computer).0
        }
        val => nu_value_to_string(&val, config, style_computer).0,
    }
}

fn try_build_map(
    record: &Record,
    span: Span,
    style_computer: &StyleComputer,
    signals: &Signals,
    config: &NuConfig,
) -> String {
    let opts = TableOpts::new(
        config,
        style_computer,
        signals,
        Span::unknown(),
        usize::MAX,
        (config.table_indent.left, config.table_indent.right),
        config.table_mode,
        0,
        false,
    );
    let result = ExpandedTable::new(None, false, String::new()).build_map(record, opts);
    match result {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => {
            nu_value_to_string(&Value::record(record.clone(), span), config, style_computer).0
        }
    }
}

fn try_build_list(
    vals: Vec<Value>,
    signals: &Signals,
    config: &NuConfig,
    span: Span,
    style_computer: &StyleComputer,
) -> String {
    let opts = TableOpts::new(
        config,
        style_computer,
        signals,
        Span::unknown(),
        usize::MAX,
        (config.table_indent.left, config.table_indent.right),
        config.table_mode,
        0,
        false,
    );

    let result = ExpandedTable::new(None, false, String::new()).build_list(&vals, opts);
    match result {
        Ok(Some(out)) => out,
        Ok(None) | Err(_) => {
            // it means that the list is empty
            nu_value_to_string(&Value::list(vals, span), config, style_computer).0
        }
    }
}
