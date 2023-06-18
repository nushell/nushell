use nu_color_config::StyleComputer;
use nu_protocol::{Span, Value};
use nu_table::{value_to_clean_styled_string, value_to_styled_string, BuildConfig, ExpandedTable};
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
            value_to_clean_styled_string(&val, config, style_computer).0
        }
        val => value_to_styled_string(&val, config, style_computer).0,
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
    let opts = BuildConfig::new(ctrlc, config, style_computer, Span::unknown(), usize::MAX);
    let result = ExpandedTable::new(None, false, String::new()).build_map(&cols, &vals, opts);
    match result {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => {
            value_to_styled_string(&Value::Record { cols, vals, span }, config, style_computer).0
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
    let opts = BuildConfig::new(ctrlc, config, style_computer, Span::unknown(), usize::MAX);
    let result = ExpandedTable::new(None, false, String::new()).build_list(&vals, opts);
    match result {
        Ok(Some(out)) => out,
        Ok(None) | Err(_) => {
            // it means that the list is empty
            value_to_styled_string(&Value::List { vals, span }, config, style_computer).0
        }
    }
}
