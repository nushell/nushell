use nu_errors::ShellError;
use nu_protocol::{did_you_mean, ColumnPath, Primitive, ShellTypeName, UntaggedValue, Value};
use nu_source::{span_for_spanned_list, Tagged};
use nu_value_ext::ValueExt;
use regex::Regex;
use std::cmp;

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    Downcase,
    Upcase,
    ToInteger,
    Substring(usize, usize),
    Replace(ReplaceAction),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReplaceAction {
    Direct(String),
    FindAndReplace(String, String),
}

#[derive(Default)]
pub struct Str {
    pub field: Option<Tagged<ColumnPath>>,
    pub error: Option<String>,
    pub action: Option<Action>,
}

impl Str {
    pub fn new() -> Self {
        Default::default()
    }

    fn apply(&self, input: &str) -> Result<UntaggedValue, ShellError> {
        let applied = match self.action.as_ref() {
            Some(Action::Downcase) => UntaggedValue::string(input.to_ascii_lowercase()),
            Some(Action::Upcase) => UntaggedValue::string(input.to_ascii_uppercase()),
            Some(Action::Substring(s, e)) => {
                let end: usize = cmp::min(*e, input.len());
                let start: usize = *s;
                if start > input.len() - 1 {
                    UntaggedValue::string("")
                } else {
                    UntaggedValue::string(
                        &input
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect::<String>(),
                    )
                }
            }
            Some(Action::Replace(mode)) => match mode {
                ReplaceAction::Direct(replacement) => UntaggedValue::string(replacement.as_str()),
                ReplaceAction::FindAndReplace(find, replacement) => {
                    let regex = Regex::new(find.as_str());

                    match regex {
                        Ok(re) => UntaggedValue::string(
                            re.replace(input, replacement.as_str()).to_owned(),
                        ),
                        Err(_) => UntaggedValue::string(input),
                    }
                }
            },
            Some(Action::ToInteger) => match input.trim() {
                other => match other.parse::<i64>() {
                    Ok(v) => UntaggedValue::int(v),
                    Err(_) => UntaggedValue::string(input),
                },
            },
            None => UntaggedValue::string(input),
        };

        Ok(applied)
    }

    pub fn for_field(&mut self, column_path: Tagged<ColumnPath>) {
        self.field = Some(column_path);
    }

    fn permit(&mut self) -> bool {
        self.action.is_none()
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    pub fn for_to_int(&mut self) {
        if self.permit() {
            self.action = Some(Action::ToInteger);
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn for_downcase(&mut self) {
        if self.permit() {
            self.action = Some(Action::Downcase);
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn for_upcase(&mut self) {
        if self.permit() {
            self.action = Some(Action::Upcase);
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn for_substring(&mut self, s: String) -> Result<(), ShellError> {
        let v: Vec<&str> = s.split(',').collect();
        let start: usize = match v[0] {
            "" => 0,
            _ => v[0]
                .trim()
                .parse()
                .map_err(|_| ShellError::untagged_runtime_error("Could not perform substring"))?,
        };
        let end: usize = match v[1] {
            "" => usize::max_value(),
            _ => v[1]
                .trim()
                .parse()
                .map_err(|_| ShellError::untagged_runtime_error("Could not perform substring"))?,
        };
        if start > end {
            self.log_error("End must be greater than or equal to Start");
        } else if self.permit() {
            self.action = Some(Action::Substring(start, end));
        } else {
            self.log_error("can only apply one");
        }

        Ok(())
    }

    pub fn for_replace(&mut self, mode: ReplaceAction) {
        if self.permit() {
            self.action = Some(Action::Replace(mode));
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn usage() -> &'static str {
        "Usage: str field [--downcase|--upcase|--to-int|--substring \"start,end\"|--replace|--find-replace [pattern replacement]]]"
    }

    pub fn strutils(&self, value: Value) -> Result<Value, ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::String(ref s)) => {
                Ok(self.apply(&s)?.into_value(value.tag()))
            }
            UntaggedValue::Primitive(Primitive::Line(ref s)) => {
                Ok(self.apply(&s)?.into_value(value.tag()))
            }
            UntaggedValue::Row(_) => match self.field {
                Some(ref f) => {
                    let fields = f.clone();

                    let replace_for =
                        value.get_data_by_column_path(
                            &f,
                            Box::new(move |(obj_source, column_path_tried, error)| {
                                match did_you_mean(&obj_source, &column_path_tried) {
                                    Some(suggestions) => ShellError::labeled_error(
                                        "Unknown column",
                                        format!("did you mean '{}'?", suggestions[0].1),
                                        span_for_spanned_list(fields.iter().map(|p| p.span)),
                                    ),
                                    None => error,
                                }
                            }),
                        );

                    let got = replace_for?;
                    let replacement = self.strutils(got)?;

                    match value
                        .replace_data_at_column_path(&f, replacement.value.into_untagged_value())
                    {
                        Some(v) => Ok(v),
                        None => Err(ShellError::labeled_error(
                            "str could not find field to replace",
                            "column name",
                            value.tag(),
                        )),
                    }
                }
                None => Err(ShellError::untagged_runtime_error(format!(
                    "{}: {}",
                    "str needs a column when applied to a value in a row",
                    Str::usage()
                ))),
            },
            _ => Err(ShellError::labeled_error(
                "Unrecognized type in stream",
                value.type_name(),
                value.tag,
            )),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::ReplaceAction;
    use super::Str;
    use nu_plugin::test_helpers::value::{int, string};

    #[test]
    fn downcases() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_downcase();
        assert_eq!(strutils.apply("ANDRES")?, string("andres").value);
        Ok(())
    }

    #[test]
    fn upcases() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_upcase();
        assert_eq!(strutils.apply("andres")?, string("ANDRES").value);
        Ok(())
    }

    #[test]
    fn converts_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_to_int();
        assert_eq!(strutils.apply("9999")?, int(9999 as i64).value);
        Ok(())
    }

    #[test]
    fn replaces() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_replace(ReplaceAction::Direct("robalino".to_string()));
        assert_eq!(strutils.apply("andres")?, string("robalino").value);
        Ok(())
    }

    #[test]
    fn find_and_replaces() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();

        strutils.for_replace(ReplaceAction::FindAndReplace(
            "kittens".to_string(),
            "jotandrehuda".to_string(),
        ));

        assert_eq!(strutils.apply("wykittens")?, string("wyjotandrehuda").value);
        Ok(())
    }
}
