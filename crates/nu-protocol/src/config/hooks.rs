use super::prelude::*;
use crate as nu_protocol;
use crate::config::{report_invalid_config_key, report_invalid_config_value};

/// Definition of a parsed hook from the config object
#[derive(Clone, Debug, IntoValue, PartialEq, Serialize, Deserialize)]
pub struct Hooks {
    pub pre_prompt: Option<Value>,
    pub pre_execution: Option<Value>,
    pub env_change: Option<Value>,
    pub display_output: Option<Value>,
    pub command_not_found: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: None,
            pre_execution: None,
            env_change: None,
            display_output: Some(Value::string(
                "if (term size).columns >= 100 { table -e } else { table }",
                Span::unknown(),
            )),
            command_not_found: None,
        }
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateFromValue for Hooks {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut Vec<ShellError>,
    ) {
        fn update_option(field: &mut Option<Value>, value: &Value) {
            if value.is_nothing() {
                *field = None;
            } else {
                *field = Some(value.clone());
            }
        }

        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "pre_prompt" => update_option(&mut self.pre_prompt, val),
                "pre_execution" => update_option(&mut self.pre_execution, val),
                "env_change" => update_option(&mut self.env_change, val),
                "display_output" => update_option(&mut self.display_output, val),
                "command_not_found" => update_option(&mut self.command_not_found, val),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
