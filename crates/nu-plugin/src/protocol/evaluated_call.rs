use nu_engine::eval_expression;
use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    FromValue, ShellError, Span, Spanned, Value,
};
use serde::{Deserialize, Serialize};

// The evaluated call is used with the Plugins because the plugin doesn't have
// access to the Stack and the EngineState. For that reason, before encoding the
// message to the plugin all the arguments to the original call (which are expressions)
// are evaluated and passed to Values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedCall {
    pub head: Span,
    pub positional: Vec<Value>,
    pub named: Vec<(Spanned<String>, Option<Value>)>,
}

impl EvaluatedCall {
    pub fn try_from_call(
        call: &Call,
        engine_state: &EngineState,
        stack: &mut Stack,
    ) -> Result<Self, ShellError> {
        let positional = call
            .positional_iter()
            .map(|expr| eval_expression(engine_state, stack, expr))
            .collect::<Result<Vec<Value>, ShellError>>()?;

        let mut named = Vec::with_capacity(call.named_len());
        for (string, _, expr) in call.named_iter() {
            let value = match expr {
                None => None,
                Some(expr) => Some(eval_expression(engine_state, stack, expr)?),
            };

            named.push((string.clone(), value))
        }

        Ok(Self {
            head: call.head,
            positional,
            named,
        })
    }

    pub fn has_flag(&self, flag_name: &str) -> bool {
        for name in &self.named {
            if flag_name == name.0.item {
                return true;
            }
        }

        false
    }

    pub fn get_flag_value(&self, flag_name: &str) -> Option<Value> {
        for name in &self.named {
            if flag_name == name.0.item {
                return name.1.clone();
            }
        }

        None
    }

    pub fn nth(&self, pos: usize) -> Option<Value> {
        self.positional.get(pos).cloned()
    }

    pub fn get_flag<T: FromValue>(&self, name: &str) -> Result<Option<T>, ShellError> {
        if let Some(value) = self.get_flag_value(name) {
            FromValue::from_value(&value).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn rest<T: FromValue>(&self, starting_pos: usize) -> Result<Vec<T>, ShellError> {
        self.positional
            .iter()
            .skip(starting_pos)
            .map(|value| FromValue::from_value(value))
            .collect()
    }

    pub fn opt<T: FromValue>(&self, pos: usize) -> Result<Option<T>, ShellError> {
        if let Some(value) = self.nth(pos) {
            FromValue::from_value(&value).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn req<T: FromValue>(&self, pos: usize) -> Result<T, ShellError> {
        if let Some(value) = self.nth(pos) {
            FromValue::from_value(&value)
        } else if self.positional.is_empty() {
            Err(ShellError::AccessEmptyContent(self.head))
        } else {
            Err(ShellError::AccessBeyondEnd(
                self.positional.len() - 1,
                self.head,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn call_to_value() {
        let call = EvaluatedCall {
            head: Span { start: 0, end: 10 },
            positional: vec![
                Value::Float {
                    val: 1.0,
                    span: Span { start: 0, end: 10 },
                },
                Value::String {
                    val: "something".into(),
                    span: Span { start: 0, end: 10 },
                },
            ],
            named: vec![
                (
                    Spanned {
                        item: "name".to_string(),
                        span: Span { start: 0, end: 10 },
                    },
                    Some(Value::Float {
                        val: 1.0,
                        span: Span { start: 0, end: 10 },
                    }),
                ),
                (
                    Spanned {
                        item: "flag".to_string(),
                        span: Span { start: 0, end: 10 },
                    },
                    None,
                ),
            ],
        };

        let name: Option<f64> = call.get_flag("name").unwrap();
        assert_eq!(name, Some(1.0));

        assert!(call.has_flag("flag"));

        let required: f64 = call.req(0).unwrap();
        assert!((required - 1.0).abs() < f64::EPSILON);

        let optional: Option<String> = call.opt(1).unwrap();
        assert_eq!(optional, Some("something".to_string()));

        let rest: Vec<String> = call.rest(1).unwrap();
        assert_eq!(rest, vec!["something".to_string()]);
    }
}
