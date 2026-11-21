use crate::{generate_strftime_list, parse_date_from_string};
use nu_engine::command_prelude::*;
use nu_protocol::DateTime;
use nu_protocol::shell_error::generic::GenericError;
use pure_rust_locales::Locale;

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
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::String)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "Lists strftime cheatsheet.", Some('l'))
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Format a given date-time using the default format (RFC 2822).",
                example: "'2021-10-22 20:00:12 +01:00' | into datetime | format date",
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
            Example {
                description: "Format a list of date strings using a given format string.",
                example: r#"["2021-10-22 20:00:12 +01:00", "2021-10-23 20:00:12 +01:00"] | format date "%Y-%m-%d""#,
                result: Some(Value::list(
                    vec![
                        Value::test_string("2021-10-22"),
                        Value::test_string("2021-10-23"),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Format a list of datetimes using a given format string.",
                example: r#"[2021-10-22T20:00:12+01:00, 2021-10-23T20:00:12+01:00] | format date "%Y-%m-%d""#,
                result: Some(Value::list(
                    vec![
                        Value::test_string("2021-10-22"),
                        Value::test_string("2021-10-23"),
                    ],
                    Span::test_data(),
                )),
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
        let locale =
            nu_protocol::time_locale(|name| stack.get_env_var(engine_state, name)?.as_str().ok());

        run(engine_state, call, input, list, format, locale)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let list = call.has_flag_const(working_set, stack, "list")?;
        let format = call.opt_const::<Spanned<String>>(working_set, stack, 0)?;
        let locale = nu_protocol::time_locale(|name| working_set.get_env_var(name)?.as_str().ok());

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
    if let PipelineData::Empty = input {
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

/// Formats `date_time` with `formatter`. The locale supplies names, `%c %x %X %r` and the
/// decimal point; see [`nu_protocol::DateTime::format`].
fn format_from(date_time: DateTime, formatter: &str, span: Span, locale: Locale) -> Value {
    match date_time.format(formatter, locale) {
        Ok(text) => Value::string(text, span),
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
                if val.year() >= 0 && val.year() <= 9999 {
                    val.to_rfc2822()
                } else {
                    return Value::error(
                        ShellError::Generic(
                            GenericError::new(
                                "Can't convert date to RFC 2822 format.",
                                "the RFC 2822 format only supports years 0 through 9999",
                                val_span,
                            )
                            .with_help(r#"use the RFC 3339 format option: "%+""#),
                        ),
                        span,
                    );
                }
            },
            span,
        ),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, val_span);
            match dt {
                Ok(x) => Value::string(
                    {
                        if x.year() >= 0 && x.year() <= 9999 {
                            x.to_rfc2822()
                        } else {
                            return Value::error(
                                ShellError::Generic(
                                    GenericError::new(
                                        "Can't convert date to RFC 2822 format.",
                                        "the RFC 2822 format only supports years 0 through 9999",
                                        val_span,
                                    )
                                    .with_help(r#"use the RFC 3339 format option: "%+""#),
                                ),
                                span,
                            );
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FormatDate)
    }

    fn date(hour: u32) -> DateTime {
        DateTime::parse_rfc3339(&format!("2026-08-27T{hour:02}:45:00Z")).expect("valid date")
    }

    /// `de_DE` and `az_IR` have no am/pm time format, and `az_IR`'s plain one carries `%O`.
    #[test]
    fn am_pm_time_falls_back_to_the_plain_time_format() {
        let span = Span::test_data();
        for locale in [Locale::de_DE, Locale::az_IR] {
            assert_eq!(
                format_from(date(13), "%r", span, locale),
                Value::string("13:45:00", span),
                "{locale:?}"
            );
        }
    }

    /// #15266: `th_TH` carries `%Ey` in its `d_fmt`, so `%x` used to fail under it.
    #[test]
    fn renders_a_date_under_a_locale_that_used_to_fail() {
        let span = Span::test_data();
        assert_eq!(
            format_from(date(1), "%x %X", span, Locale::th_TH),
            Value::string("27/08/26 01:45:00", span),
            "th_TH must render, and must keep its own day/month order"
        );
        assert_eq!(
            format_from(date(1), "%x %X", span, Locale::en_US),
            Value::string("08/27/2026 01:45:00 AM", span)
        );
    }
}
