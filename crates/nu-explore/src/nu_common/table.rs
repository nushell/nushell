use nu_color_config::StyleComputer;
use nu_protocol::{Span, Value};
use nu_table::{
    common::{nu_value_to_string, nu_value_to_string_clean},
    ExpandedTable, TableOpts,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::nu_common::NuConfig;

pub fn try_build_table(
    ctrlc: Option<Arc<AtomicBool>>,
    config: &NuConfig,
    style_computer: &StyleComputer,
    value: Value,
) -> String {
    match value {
        Value::List { vals, span } => try_build_list(vals, ctrlc, config, span, style_computer),
        Value::Record { cols, vals, span } => {
            try_build_map(cols, vals, span, style_computer, ctrlc, config)
        }
        val if matches!(val, Value::String { .. }) => {
            nu_value_to_string_clean(&val, config, style_computer).0
        }
        val => nu_value_to_string(&val, config, style_computer).0,
    }
}

fn try_build_map(
    cols: Vec<String>,
    vals: Vec<Value>,
    span: Span,
    style_computer: &StyleComputer,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &NuConfig,
) -> String {
    let opts = TableOpts::new(
        config,
        style_computer,
        ctrlc,
        Span::unknown(),
        0,
        usize::MAX,
    );
    let result = ExpandedTable::new(None, false, String::new()).build_map(&cols, &vals, opts);
    match result {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => {
            nu_value_to_string(&Value::Record { cols, vals, span }, config, style_computer).0
        }
    }
}

fn try_build_list(
    vals: Vec<Value>,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &NuConfig,
    span: Span,
    style_computer: &StyleComputer,
) -> String {
    let opts = TableOpts::new(
        config,
        style_computer,
        ctrlc,
        Span::unknown(),
        0,
        usize::MAX,
    );
    let result = ExpandedTable::new(None, false, String::new()).build_list(&vals, opts);
    match result {
        Ok(Some(out)) => out,
        Ok(None) | Err(_) => {
            // it means that the list is empty
            nu_value_to_string(&Value::List { vals, span }, config, style_computer).0
        }
    }
}
