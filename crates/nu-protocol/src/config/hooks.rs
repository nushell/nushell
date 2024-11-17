use super::prelude::*;
use crate::{self as nu_protocol, engine::Closure, IntoSpanned, Record, Spanned};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookCode {
    Code(String),
    Closure(Closure),
}

impl IntoValue for HookCode {
    fn into_value(self, span: Span) -> Value {
        match self {
            HookCode::Code(code) => code.into_value(span),
            HookCode::Closure(closure) => closure.into_value(span),
        }
    }
}

#[derive(Debug, Clone, IntoValue, Serialize, Deserialize)]
pub struct ConditionalHook {
    pub condition: Option<Spanned<Closure>>,
    pub code: Spanned<HookCode>,
}

impl ConditionalHook {
    pub(crate) fn from_record<'a>(
        record: &'a Record,
        span: Span,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) -> Option<Self> {
        let mut condition = None;
        let mut code = None;
        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "condition" => match val {
                    Value::Closure { val, .. } => {
                        condition = Some(val.as_ref().clone().into_spanned(span));
                    }
                    Value::Nothing { .. } => {
                        condition = None;
                    }
                    val => {
                        errors.type_mismatch(path, Type::custom("closure or nothing"), val);
                    }
                },
                "code" => match val {
                    Value::String { val, .. } => {
                        code = Some(HookCode::Code(val.clone()).into_spanned(span));
                    }
                    Value::Closure { val, .. } => {
                        code = Some(HookCode::Closure(val.as_ref().clone()).into_spanned(span));
                    }
                    val => {
                        errors.type_mismatch(path, Type::custom("string or closure"), val);
                    }
                },
                _ => errors.unknown_option(path, val),
            }
        }
        if let Some(code) = code {
            Some(ConditionalHook { condition, code })
        } else {
            errors.missing_column(path, "code", span);
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Hook {
    Unconditional(Spanned<HookCode>),
    Conditional(ConditionalHook),
}

impl IntoValue for Hook {
    fn into_value(self, span: Span) -> Value {
        match self {
            Self::Unconditional(code) => code.into_value(span),
            Self::Conditional(hook) => hook.into_value(span),
        }
    }
}

impl Hook {
    pub(crate) fn from_value<'a>(
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) -> Option<Self> {
        match value {
            Value::String { val, .. } => Some(Hook::Unconditional(
                HookCode::Code(val.clone()).into_spanned(value.span()),
            )),
            Value::Closure { val, .. } => Some(Hook::Unconditional(
                HookCode::Closure(val.as_ref().clone()).into_spanned(value.span()),
            )),
            Value::Record { val: record, .. } => {
                ConditionalHook::from_record(record, value.span(), path, errors)
                    .map(Hook::Conditional)
            }
            val => {
                errors.type_mismatch(path, Type::custom("string, record, or closure"), val);
                None
            }
        }
    }
}

/// Definition of a parsed hook from the config object
#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct Hooks {
    pub pre_prompt: Vec<Hook>,
    pub pre_execution: Vec<Hook>,
    pub env_change: HashMap<String, Vec<Hook>>,
    pub display_output: Option<Hook>,
    pub command_not_found: Option<Hook>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            pre_prompt: Vec::new(),
            pre_execution: Vec::new(),
            env_change: HashMap::new(),
            display_output: Some(Hook::Unconditional(
                HookCode::Code("if (term size).columns >= 100 { table -e } else { table }".into())
                    .into_spanned(Span::unknown()),
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
        fn update_hook<'a>(
            field: &mut Option<Hook>,
            value: &'a Value,
            path: &mut ConfigPath<'a>,
            errors: &mut ConfigErrors,
        ) {
            match value {
                Value::String { .. } | Value::Closure { .. } | Value::Record { .. } => {
                    if let Some(hook) = Hook::from_value(value, path, errors) {
                        *field = Some(hook);
                    }
                }
                Value::Nothing { .. } => *field = None,
                val => errors.type_mismatch(
                    path,
                    Type::custom("string, closure, record, or nothing"),
                    val,
                ),
            }
        }

        fn update_hook_list<'a>(
            field: &mut Vec<Hook>,
            val: &'a Value,
            path: &mut ConfigPath<'a>,
            errors: &mut ConfigErrors,
        ) {
            if let Ok(hooks) = val.as_list() {
                *field = hooks
                    .iter()
                    .enumerate()
                    .filter_map(|(i, val)| {
                        let path = &mut path.push_row(i);
                        Hook::from_value(val, path, errors)
                    })
                    .collect()
            } else {
                errors.type_mismatch(path, Type::list(Type::Any), val)
            }
        }

        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "pre_prompt" => update_hook_list(&mut self.pre_prompt, val, path, errors),
                "pre_execution" => update_hook_list(&mut self.pre_execution, val, path, errors),
                "env_change" => {
                    if let Ok(record) = val.as_record() {
                        self.env_change = record
                            .iter()
                            .filter_map(|(key, val)| {
                                let path = &mut path.push(key);
                                if let Ok(hooks) = val.as_list() {
                                    let hooks = hooks
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, val)| {
                                            let path = &mut path.push_row(i);
                                            Hook::from_value(val, path, errors)
                                        })
                                        .collect();
                                    Some((key.clone(), hooks))
                                } else {
                                    errors.type_mismatch(path, Type::list(Type::Any), val);
                                    None
                                }
                            })
                            .collect()
                    } else {
                        errors.type_mismatch(path, Type::record(), val)
                    }
                }
                "display_output" => update_hook(&mut self.display_output, val, path, errors),
                "command_not_found" => update_hook(&mut self.command_not_found, val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
