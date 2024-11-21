use super::prelude::*;
use crate as nu_protocol;
use crate::Record;

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
            pre_prompt: Some(Value::list(vec![], Span::unknown())),
            pre_execution: Some(Value::list(vec![], Span::unknown())),
            env_change: Some(Value::record(Record::default(), Span::unknown())),
            display_output: Some(Value::string(
                "if (term size).columns >= 100 { table -e } else { table }",
                Span::unknown(),
            )),
            command_not_found: Some(Value::list(vec![], Span::unknown())),
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
        errors: &mut ConfigErrors,
    ) {
        fn update_option(field: &mut Option<Value>, value: &Value) {
            if value.is_nothing() {
                *field = None;
            } else {
                *field = Some(value.clone());
            }
        }

        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "pre_prompt" => update_option(&mut self.pre_prompt, val),
                "pre_execution" => update_option(&mut self.pre_execution, val),
                "env_change" => update_option(&mut self.env_change, val),
                "display_output" => update_option(&mut self.display_output, val),
                "command_not_found" => update_option(&mut self.command_not_found, val),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
