use nu_plugin::plugin::PluginError;
use nu_protocol::{Span, Value};

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
    pub action: Option<Action>,
}

impl Inc {
    pub fn new() -> Self {
        Default::default()
    }

    fn apply(&self, input: &str) -> Value {
        match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => {
                        return Value::String {
                            val: input.to_string(),
                            span: Span::unknown(),
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
                    span: Span::unknown(),
                }
            }
            Some(Action::Default) | None => match input.parse::<u64>() {
                Ok(v) => Value::String {
                    val: (v + 1).to_string(),
                    span: Span::unknown(),
                },
                Err(_) => Value::String {
                    val: input.to_string(),
                    span: Span::unknown(),
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

    pub fn inc(&self, value: &Value) -> Result<Value, PluginError> {
        match value {
            Value::Int { val, span } => Ok(Value::Int {
                val: val + 1,
                span: *span,
            }),
            Value::String { val, .. } => Ok(self.apply(val)),
            _ => Err(PluginError::RunTimeError("incrementable value".to_string())),
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
                span: Span::unknown(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Major);
            assert_eq!(inc.apply("0.1.3"), expected)
        }

        #[test]
        fn minor() {
            let expected = Value::String {
                val: "0.2.0".to_string(),
                span: Span::unknown(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Minor);
            assert_eq!(inc.apply("0.1.3"), expected)
        }

        #[test]
        fn patch() {
            let expected = Value::String {
                val: "0.1.4".to_string(),
                span: Span::unknown(),
            };
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Patch);
            assert_eq!(inc.apply("0.1.3"), expected)
        }
    }
}
