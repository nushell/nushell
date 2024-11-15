use super::prelude::*;
use crate as nu_protocol;
use std::collections::HashMap;

/// Definition of a parsed hook from the config object
#[derive(Clone, Debug, IntoValue, PartialEq, Serialize, Deserialize)]
pub struct Hooks {
    pub pre_prompt: Vec<Value>,
    pub pre_execution: Vec<Value>,
    pub env_change: HashMap<String, Value>,
    pub display_output: Option<Value>,
    pub command_not_found: Option<Value>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: Vec::new(),
            pre_execution: Vec::new(),
            env_change: HashMap::new(),
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
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "pre_prompt" => {
                    if let Ok(hooks) = val.as_list() {
                        self.pre_prompt = hooks.into()
                    } else {
                        errors.type_mismatch(path, Type::list(Type::Any), val);
                    }
                }
                "pre_execution" => {
                    if let Ok(hooks) = val.as_list() {
                        self.pre_execution = hooks.into()
                    } else {
                        errors.type_mismatch(path, Type::list(Type::Any), val);
                    }
                }
                "env_change" => self.env_change.update(val, path, errors),
                "display_output" => {
                    self.display_output = if val.is_nothing() {
                        None
                    } else {
                        Some(val.clone())
                    }
                }
                "command_not_found" => {
                    self.command_not_found = if val.is_nothing() {
                        None
                    } else {
                        Some(val.clone())
                    }
                }
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
