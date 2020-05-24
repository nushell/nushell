extern crate chrono;

use bigdecimal::BigDecimal;
use chrono::DateTime;
use nu_errors::ShellError;
use nu_protocol::{did_you_mean, ColumnPath, Primitive, ShellTypeName, UntaggedValue, Value};
use nu_source::{span_for_spanned_list, Tagged};
use nu_value_ext::ValueExt;
use regex::Regex;
use std::cmp;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    Capitalize,
    Downcase,
    Upcase,
    ToInteger,
    ToFloat,
    Substring(usize, usize),
    Replace(ReplaceAction),
    ToDateTime(String),
    Trim,
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
            Some(Action::Trim) => UntaggedValue::string(input.trim()),
            Some(Action::Capitalize) => {
                let mut capitalized = String::new();

                for (idx, character) in input.chars().enumerate() {
                    let out = if idx == 0 {
                        character.to_uppercase().to_string()
                    } else {
                        character.to_lowercase().to_string()
                    };

                    capitalized.push_str(&out);
                }

                UntaggedValue::string(capitalized)
            }
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
            Some(Action::ToInteger) => {
                let other = input.trim();
                match other.parse::<i64>() {
                    Ok(v) => UntaggedValue::int(v),
                    Err(_) => UntaggedValue::string(input),
                }
            }
            Some(Action::ToFloat) => match BigDecimal::from_str(input.trim()) {
                Ok(v) => UntaggedValue::decimal(v),
                Err(_) => UntaggedValue::string(input),
            },
            Some(Action::ToDateTime(dt)) => match DateTime::parse_from_str(input, dt) {
                Ok(d) => UntaggedValue::date(d),
                Err(_) => UntaggedValue::string(input),
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
        self.add_action(Action::ToInteger);
    }

    pub fn for_to_float(&mut self) {
        self.add_action(Action::ToFloat);
    }

    pub fn for_capitalize(&mut self) {
        self.add_action(Action::Capitalize);
    }

    pub fn for_trim(&mut self) {
        self.add_action(Action::Trim);
    }

    pub fn for_downcase(&mut self) {
        self.add_action(Action::Downcase);
    }

    pub fn for_upcase(&mut self) {
        self.add_action(Action::Upcase);
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
        } else {
            self.add_action(Action::Substring(start, end));
        }

        Ok(())
    }

    pub fn for_replace(&mut self, mode: ReplaceAction) {
        self.add_action(Action::Replace(mode));
    }

    pub fn for_date_time(&mut self, dt: String) {
        self.add_action(Action::ToDateTime(dt));
    }

    fn add_action(&mut self, act: Action) {
        if self.permit() {
            self.action = Some(act);
        } else {
            self.log_error("can only apply one");
        }
    }

    pub fn usage() -> &'static str {
        "Usage: str field [--capitalize|--downcase|--upcase|--to-int|--to-float|--substring \"start,end\"|--replace|--find-replace [pattern replacement]|to-date-time|--trim]"
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
    use nu_plugin::test_helpers::value::{decimal, int, string};

    #[test]
    fn trim() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_trim();
        assert_eq!(strutils.apply("andres ")?, string("andres").value);
        Ok(())
    }

    #[test]
    fn capitalize() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_capitalize();
        assert_eq!(strutils.apply("andres")?, string("Andres").value);
        Ok(())
    }

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
    fn converts_to_float() -> Result<(), Box<dyn std::error::Error>> {
        let mut strutils = Str::new();
        strutils.for_to_float();
        assert_eq!(strutils.apply("3.1415")?, decimal(3.1415).value);
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
