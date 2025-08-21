use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{DateTime, Datelike, Locale, TimeZone};
use nu_engine::command_prelude::*;

use nu_utils::locale::{LOCALE_OVERRIDE_ENV_VAR, get_system_locale_string};
use std::fmt::{Display, Write};

#[derive(Clone)]
pub struct FormatDate;

impl Command for FormatDate {
    fn name(&self) -> &str {
        "format date"
    }

    fn signature(&self) -> Signature {
        Signature::build("format date")
            .input_output_types(vec![
                (Type::Date, Type::String),
                (Type::String, Type::String),
                (Type::Nothing, Type::table()),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                // only applicable for --list flag
                (Type::Any, Type::table()),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "lists strftime cheatsheet", Some('l'))
            .optional(
                "format string",
                SyntaxShape::String,
                "The desired format date.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Format a given date using a format string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fmt", "strftime"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Format a given date-time using the default format (RFC 2822).",
                example: r#"'2021-10-22 20:00:12 +01:00' | into datetime | format date"#,
                result: Some(Value::string(
                    "Fri, 22 Oct 2021 20:00:12 +0100".to_string(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Format a given date-time as a string using the default format (RFC 2822).",
                example: r#""2021-10-22 20:00:12 +01:00" | format date"#,
                result: Some(Value::string(
                    "Fri, 22 Oct 2021 20:00:12 +0100".to_string(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Format a given date-time according to the RFC 3339 standard.",
                example: r#"'2021-10-22 20:00:12 +01:00' | into datetime | format date "%+""#,
                result: Some(Value::string(
                    "2021-10-22T20:00:12+01:00".to_string(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Format the current date-time using a given format string.",
                example: r#"date now | format date "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Format the current date using a given format string.",
                example: r#"date now | format date "%Y-%m-%d %H:%M:%S""#,
                result: None,
            },
            Example {
                description: "Format a given date using a given format string.",
                example: r#""2021-10-22 20:00:12 +01:00" | format date "%Y-%m-%d""#,
                result: Some(Value::test_string("2021-10-22")),
            },
        ]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list = call.has_flag(engine_state, stack, "list")?;
        let format = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        // env var preference is documented at https://www.gnu.org/software/gettext/manual/html_node/Locale-Environment-Variables.html
        // LC_ALL ovverides LC_TIME, LC_TIME overrides LANG

        // get the locale first so we can use the proper get_env_var functions since this is a const command
        // we can override the locale by setting $env.NU_TEST_LOCALE_OVERRIDE or $env.LC_TIME
        let locale = if let Some(loc) = engine_state
            .get_env_var(LOCALE_OVERRIDE_ENV_VAR)
            .or_else(|| engine_state.get_env_var("LC_ALL"))
            .or_else(|| engine_state.get_env_var("LC_TIME"))
            .or_else(|| engine_state.get_env_var("LANG"))
        {
            let locale_str = loc.as_str()?.split('.').next().unwrap_or("en_US");
            locale_str.try_into().unwrap_or(Locale::en_US)
        } else {
            get_system_locale_string()
                .map(|l| l.replace('-', "_"))
                .unwrap_or_else(|| String::from("en_US"))
                .as_str()
                .try_into()
                .unwrap_or(Locale::en_US)
        };

        run(engine_state, call, input, list, format, locale)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list = call.has_flag_const(working_set, "list")?;
        let format = call.opt_const::<Spanned<String>>(working_set, 0)?;

        // env var preference is documented at https://www.gnu.org/software/gettext/manual/html_node/Locale-Environment-Variables.html
        // LC_ALL ovverides LC_TIME, LC_TIME overrides LANG

        // get the locale first so we can use the proper get_env_var functions since this is a const command
        // we can override the locale by setting $env.NU_TEST_LOCALE_OVERRIDE or $env.LC_TIME
        let locale = if let Some(loc) = working_set
            .get_env_var(LOCALE_OVERRIDE_ENV_VAR)
            .or_else(|| working_set.get_env_var("LC_ALL"))
            .or_else(|| working_set.get_env_var("LC_TIME"))
            .or_else(|| working_set.get_env_var("LANG"))
        {
            let locale_str = loc.as_str()?.split('.').next().unwrap_or("en_US");
            locale_str.try_into().unwrap_or(Locale::en_US)
        } else {
            get_system_locale_string()
                .map(|l| l.replace('-', "_"))
                .unwrap_or_else(|| String::from("en_US"))
                .as_str()
                .try_into()
                .unwrap_or(Locale::en_US)
        };

        run(working_set.permanent(), call, input, list, format, locale)
    }
}

fn run(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    list: bool,
    format: Option<Spanned<String>>,
    locale: Locale,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    if list {
        return Ok(PipelineData::value(
            generate_strftime_list(head, false),
            None,
        ));
    }

    // This doesn't match explicit nulls
    if matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }
    input.map(
        move |value| match &format {
            Some(format) => format_helper(value, format.item.as_str(), format.span, head, locale),
            None => format_helper_rfc2822(value, head),
        },
        engine_state.signals(),
    )
}

fn format_from<Tz: TimeZone>(
    date_time: DateTime<Tz>,
    formatter: &str,
    span: Span,
    locale: Locale,
) -> Value
where
    Tz::Offset: Display,
{
    let mut formatter_buf = String::new();
    let format = date_time.format_localized(formatter, locale);

    match formatter_buf.write_fmt(format_args!("{format}")) {
        Ok(_) => Value::string(formatter_buf, span),
        Err(_) => Value::error(
            ShellError::TypeMismatch {
                err_message: "invalid format".to_string(),
                span,
            },
            span,
        ),
    }
}

fn format_helper(
    value: Value,
    formatter: &str,
    formatter_span: Span,
    head_span: Span,
    locale: Locale,
) -> Value {
    match value {
        Value::Date { val, .. } => format_from(val, formatter, formatter_span, locale),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, formatter_span);

            match dt {
                Ok(x) => format_from(x, formatter, formatter_span, locale),
                Err(e) => e,
            }
        }
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "date, string (that represents datetime)".into(),
                wrong_type: value.get_type().to_string(),
                dst_span: head_span,
                src_span: value.span(),
            },
            head_span,
        ),
    }
}

fn format_helper_rfc2822(value: Value, span: Span) -> Value {
    let val_span = value.span();
    match value {
        Value::Date { val, .. } => Value::string(
            {
                if val.year() >= 0 {
                    val.to_rfc2822()
                } else {
                    val.to_rfc3339()
                }
            },
            span,
        ),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, val_span);
            match dt {
                Ok(x) => Value::string(
                    {
                        if x.year() >= 0 {
                            x.to_rfc2822()
                        } else {
                            x.to_rfc3339()
                        }
                    },
                    span,
                ),
                Err(e) => e,
            }
        }
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "date, string (that represents datetime)".into(),
                wrong_type: value.get_type().to_string(),
                dst_span: span,
                src_span: val_span,
            },
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FormatDate {})
    }
}
