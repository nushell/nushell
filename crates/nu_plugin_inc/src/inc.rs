use nu_errors::ShellError;
use nu_protocol::{did_you_mean, ColumnPath, Primitive, ShellTypeName, UntaggedValue, Value};
use nu_source::{span_for_spanned_list, HasSpan, SpannedItem, Tagged};
use nu_value_ext::{get_data_by_column_path, ValueExt};

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
    pub field: Option<Tagged<ColumnPath>>,
    pub error: Option<String>,
    pub action: Option<Action>,
}

impl Inc {
    pub fn new() -> Self {
        Default::default()
    }

    fn apply(&self, input: &str) -> UntaggedValue {
        match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(&input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => return UntaggedValue::string(input.to_string()),
                };

                match act_on {
                    SemVerAction::Major => ver.increment_major(),
                    SemVerAction::Minor => ver.increment_minor(),
                    SemVerAction::Patch => ver.increment_patch(),
                }

                UntaggedValue::string(ver.to_string())
            }
            Some(Action::Default) | None => match input.parse::<u64>() {
                Ok(v) => UntaggedValue::string(format!("{}", v + 1)),
                Err(_) => UntaggedValue::string(input),
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

    pub fn inc(&self, value: Value) -> Result<Value, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(i)) => {
                Ok(UntaggedValue::int(i + 1).into_value(value.tag()))
            }
            UntaggedValue::Primitive(Primitive::Filesize(b)) => {
                Ok(UntaggedValue::filesize(b + 1_u64).into_value(value.tag()))
            }
            UntaggedValue::Primitive(Primitive::String(ref s)) => {
                Ok(self.apply(&s).into_value(value.tag()))
            }
            UntaggedValue::Table(values) => {
                if values.len() == 1 {
                    Ok(UntaggedValue::Table(vec![self.inc(values[0].clone())?])
                        .into_value(value.tag()))
                } else {
                    Err(ShellError::type_error(
                        "incrementable value",
                        value.type_name().spanned(value.span()),
                    ))
                }
            }

            UntaggedValue::Row(_) => match self.field {
                Some(ref f) => {
                    let fields = f.clone();

                    let replace_for = get_data_by_column_path(
                        &value,
                        &f,
                        move |obj_source, column_path_tried, _| match did_you_mean(
                            &obj_source,
                            column_path_tried.as_string(),
                        ) {
                            Some(suggestions) => ShellError::labeled_error(
                                "Unknown column",
                                format!("did you mean '{}'?", suggestions[0]),
                                span_for_spanned_list(fields.iter().map(|p| p.span)),
                            ),
                            None => ShellError::labeled_error(
                                "Unknown column",
                                "row does not contain this column",
                                span_for_spanned_list(fields.iter().map(|p| p.span)),
                            ),
                        },
                    );

                    let got = replace_for?;
                    let replacement = self.inc(got)?;

                    value
                        .replace_data_at_column_path(&f, replacement.value.into_untagged_value())
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "inc could not find field to replace",
                                "column name",
                                value.tag(),
                            )
                        })
                }
                None => Err(ShellError::untagged_runtime_error(
                    "inc needs a field when incrementing a column in a table",
                )),
            },
            _ => Err(ShellError::type_error(
                "incrementable value",
                value.type_name().spanned(value.span()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    mod semver {
        use crate::inc::SemVerAction;
        use crate::Inc;
        use nu_test_support::value::string;

        #[test]
        fn major() {
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Major);
            assert_eq!(inc.apply("0.1.3"), string("1.0.0").value);
        }

        #[test]
        fn minor() {
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Minor);
            assert_eq!(inc.apply("0.1.3"), string("0.2.0").value);
        }

        #[test]
        fn patch() {
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Patch);
            assert_eq!(inc.apply("0.1.3"), string("0.1.4").value);
        }
    }
}
