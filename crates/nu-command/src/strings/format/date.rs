use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{DateTime, Locale, TimeZone};
use nu_engine::command_prelude::*;

use nu_utils::locale::{get_system_locale_string, LOCALE_OVERRIDE_ENV_VAR};
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
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .switch("list", "lists strftime cheatsheet", Some('l'))
            .optional(
                "format string",
                SyntaxShape::String,
                "The desired format date.",
            )
            .category(Category::Date)
    }

    fn usage(&self) -> &str {
        "Format a given date using a format string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["fmt", "strftime"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        if call.has_flag(engine_state, stack, "list")? {
            return Ok(PipelineData::Value(
                generate_strftime_list(head, false),
                None,
            ));
        }

        let format = call.opt::<Spanned<String>>(engine_state, stack, 0)?;

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| match &format {
                Some(format) => format_helper(value, format.item.as_str(), format.span, head),
                None => format_helper_rfc2822(value, head),
            },
            engine_state.ctrlc.clone(),
        )
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
                description:
                    "Format a given date-time as a string using the default format (RFC 2822).",
                example: r#""2021-10-22 20:00:12 +01:00" | format date"#,
                result: Some(Value::string(
                    "Fri, 22 Oct 2021 20:00:12 +0100".to_string(),
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
}

fn format_from<Tz: TimeZone>(date_time: DateTime<Tz>, formatter: &str, span: Span) -> Value
where
    Tz::Offset: Display,
{
    let mut formatter_buf = String::new();
    // Format using locale LC_TIME
    let locale = if let Ok(l) =
        std::env::var(LOCALE_OVERRIDE_ENV_VAR).or_else(|_| std::env::var("LC_TIME"))
    {
        let locale_str = l.split('.').next().unwrap_or("en_US");
        locale_str.try_into().unwrap_or(Locale::en_US)
    } else {
        // LC_ALL > LC_CTYPE > LANG
        // Not locale present, default to en_US
        get_system_locale_string()
            .map(|l| l.replace('-', "_")) // `chrono::Locale` needs something like `xx_xx`, rather than `xx-xx`
            .unwrap_or_else(|| String::from("en_US"))
            .as_str()
            .try_into()
            .unwrap_or(Locale::en_US)
    };
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

fn format_helper(value: Value, formatter: &str, formatter_span: Span, head_span: Span) -> Value {
    match value {
        Value::Date { val, .. } => format_from(val, formatter, formatter_span),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, formatter_span);

            match dt {
                Ok(x) => format_from(x, formatter, formatter_span),
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
        Value::Date { val, .. } => Value::string(val.to_rfc2822(), span),
        Value::String { val, .. } => {
            let dt = parse_date_from_string(&val, val_span);
            match dt {
                Ok(x) => Value::string(x.to_rfc2822(), span),
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
