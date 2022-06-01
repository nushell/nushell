use nu_plugin::LabeledError;
use nu_protocol::{ast::CellPath, Span, Value};

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    SemVerAction(SemVerAction),
    Default,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SemVerAction {
    Major,
    Minor,
    Patch,
}

#[derive(Default)]
pub struct Inc {
    pub error: Option<String>,
    pub cell_path: Option<CellPath>,
    pub action: Option<Action>,
}

impl Inc {
    pub fn new() -> Self {
        Default::default()
    }

    fn apply(&self, input: &str, head: Span) -> Value {
        match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => {
                        return Value::String {
                            val: input.to_string(),
                            span: head,
                        }
                    }
                };

                match act_on {
                    SemVerAction::Major => ver.increment_major(),
                    SemVerAction::Minor => ver.increment_minor(),
                    SemVerAction::Patch => ver.increment_patch(),
                }

                Value::String {
                    val: ver.to_string(),
                    span: head,
                }
            }
            Some(Action::Default) | None => match input.parse::<u64>() {
                Ok(v) => Value::String {
                    val: (v + 1).to_string(),
                    span: head,
                },
                Err(_) => Value::String {
                    val: input.to_string(),
                    span: head,
                },
            },
        }
    }

    pub fn for_semver(&mut self, part: SemVerAction) {
        if self.permit() {
            self.action = Some(Action::SemVerAction(part));
        } else {
            self.log_error("can only apply one");
        }
    }

    fn permit(&mut self) -> bool {
        self.action.is_none()
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    pub fn usage() -> &'static str {
        "Usage: inc field [--major|--minor|--patch]"
    }

    pub fn inc(&self, head: Span, value: &Value) -> Result<Value, LabeledError> {
        if let Some(cell_path) = &self.cell_path {
            let working_value = value.clone();
            let cell_value = working_value.follow_cell_path(&cell_path.members, false)?;

            let cell_value = self.inc_value(head, &cell_value)?;

            let mut value = value.clone();
            value
                .update_data_at_cell_path(&cell_path.members, cell_value)
                .map_err(|x| {
                    let error: LabeledError = x.into();
                    error
                })?;
            Ok(value)
        } else {
            self.inc_value(head, value)
        }
    }

    pub fn inc_value(&self, head: Span, value: &Value) -> Result<Value, LabeledError> {
        match value {
            Value::Int { val, span } => Ok(Value::Int {
                val: val + 1,
                span: *span,
            }),
            Value::String { val, .. } => Ok(self.apply(val, head)),
            x => {
                let msg = x.as_string().map_err(|e| LabeledError {
                    label: "Unable to extract string".into(),
                    msg: format!("value cannot be converted to string {:?} - {}", x, e),
                    span: Some(head),
                })?;

                Err(LabeledError {
                    label: "Incorrect value".into(),
                    msg,
                    span: Some(head),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    mod semver {
        use nu_protocol::{Span, Value};

        use crate::inc::SemVerAction;
        use crate::Inc;

        #[test]
        fn major() {
            let expected = Value::String {
                val: "1.0.0".to_string(),
                span: Span::test_data(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Major);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }

        #[test]
        fn minor() {
            let expected = Value::String {
                val: "0.2.0".to_string(),
                span: Span::test_data(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Minor);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }

        #[test]
        fn patch() {
            let expected = Value::String {
                val: "0.1.4".to_string(),
                span: Span::test_data(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Patch);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }
    }
}
