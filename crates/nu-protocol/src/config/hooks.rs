use super::prelude::*;
use crate::engine::EnvName;
use std::collections::HashMap;

/// Definition of a parsed hook from the config object
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hooks {
    pub pre_prompt: Vec<Value>,
    pub pre_execution: Vec<Value>,
    pub env_change: HashMap<EnvName, Vec<Value>>,
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

impl IntoValue for Hooks {
    fn into_value(self, span: Span) -> Value {
        let env_change = self
            .env_change
            .into_iter()
            .map(|(key, value)| (key.into_string(), value.into_value(span)))
            .collect::<crate::Record>();

        record! {
            "pre_prompt" => self.pre_prompt.into_value(span),
            "pre_execution" => self.pre_execution.into_value(span),
            "env_change" => env_change.into_value(span),
            "display_output" => self.display_output.into_value(span),
            "command_not_found" => self.command_not_found.into_value(span),
        }
        .into_value(span)
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
                "env_change" => {
                    if let Ok(record) = val.as_record() {
                        self.env_change = record
                            .iter()
                            .map(|(key, val)| {
                                let old = self
                                    .env_change
                                    .remove(&EnvName::from(key))
                                    .unwrap_or_default();
                                let new = if let Ok(hooks) = val.as_list() {
                                    hooks.into()
                                } else {
                                    errors.type_mismatch(
                                        &path.push(key),
                                        Type::list(Type::Any),
                                        val,
                                    );
                                    old
                                };
                                (key.as_str().into(), new)
                            })
                            .collect();
                    } else {
                        errors.type_mismatch(path, Type::record(), val);
                    }
                }
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
