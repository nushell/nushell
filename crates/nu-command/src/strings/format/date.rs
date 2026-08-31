use crate::{generate_strftime_list, parse_date_from_string};
use chrono::{DateTime, Datelike, Locale, TimeZone};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use pure_rust_locales::locale_match;

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
        let locale = get_locale(|name| stack.get_env_var(engine_state, name)?.as_str().ok());

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
        let locale = get_locale(|name| working_set.get_env_var(name)?.as_str().ok());

        run(working_set.permanent(), call, input, list, format, locale)
    }
}

fn get_locale<'a, F>(env_getter: F) -> Locale
where
    F: Fn(&str) -> Option<&'a str> + 'a,
{
    nu_utils::get_locale_from_env_vars(Some("LC_TIME"), env_getter)
        .and_then(|s| Locale::try_from(s.as_ref()).ok())
        .unwrap_or(Locale::en_US)
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

/// Expand the locale-dependent specifiers ourselves, with `%E`/`%O` removed.
///
/// chrono expands `%x`, `%X`, `%c` and `%r` from the locale's own format
/// strings while rendering, so a modifier the user never typed can appear at
/// that point. `%E` and `%O` select an "alternative representation" that POSIX
/// says to ignore when the implementation has none, but chrono implements
/// neither and reports a format error instead. `th_TH` and `lo_LA` both carry
/// `%Ey` (Buddhist era) in their `d_fmt`, so every `format date` under those
/// locales failed, including the one in `default_env.nu` that greets you at
/// startup (#15266).
///
/// Doing the expansion here keeps the locale's own field order — `th_TH` still
/// renders `27/08/26` rather than the `08/27/26` a fallback locale would give —
/// and only drops the era representation chrono cannot produce.
fn resolve_locale_specifiers(formatter: &str, locale: Locale) -> String {
    // Expand first, then strip: the expansion is what introduces `%Ey`.
    // Strip before as well, so `%Ex` still reaches the expansion as `%x`.
    let stripped = strip_alternative_modifiers(formatter);
    let expanded = expand_locale_specifiers(&stripped, locale);
    strip_alternative_modifiers(&expanded)
}

fn expand_locale_specifiers(formatter: &str, locale: Locale) -> String {
    let mut out = String::with_capacity(formatter.len());
    let mut chars = formatter.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // `%%` is a literal percent, not the start of a specifier.
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push_str("%%");
            continue;
        }

        let expansion = match chars.peek() {
            Some('x') => Some(locale_match!(locale => LC_TIME::D_FMT)),
            Some('X') => Some(locale_match!(locale => LC_TIME::T_FMT)),
            Some('c') => Some(locale_match!(locale => LC_TIME::D_T_FMT)),
            Some('r') => {
                // A locale can have no am/pm form at all — `t_fmt_ampm` is ""
                // for `de_DE`, `fr_FR`, `nl_NL`, `az_IR` and `fa_IR`. chrono
                // falls back to the plain time format there, so do that same
                // substitution here: handing `%r` back to chrono would let its
                // fallback reintroduce the modifier we are removing, which is
                // what left `az_IR`/`fa_IR` (`t_fmt` = `%OH:%OM:%OS`) failing.
                let ampm = locale_match!(locale => LC_TIME::T_FMT_AMPM);
                Some(if ampm.is_empty() {
                    locale_match!(locale => LC_TIME::T_FMT)
                } else {
                    ampm
                })
            }
            _ => None,
        };

        // An empty expansion would erase the output, so leave the specifier
        // alone and let chrono resolve it.
        match expansion {
            Some(fmt) if !fmt.is_empty() => {
                chars.next();
                out.push_str(fmt);
            }
            _ => out.push('%'),
        }
    }

    out
}

fn strip_alternative_modifiers(formatter: &str) -> String {
    let mut out = String::with_capacity(formatter.len());
    let mut chars = formatter.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('%') => {
                chars.next();
                out.push_str("%%");
            }
            // `%Ey` becomes `%y`, `%Od` becomes `%d`: the specifier stays, the
            // alternative representation is what goes.
            Some('E') | Some('O') => {
                chars.next();
                out.push('%');
            }
            _ => out.push('%'),
        }
    }

    out
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
    // Handle custom format specifiers for compact formats
    let processed_formatter = formatter
        .replace("%J", "%Y%m%d") // %J for joined date (YYYYMMDD)
        .replace("%Q", "%H%M%S"); // %Q for sequential time (HHMMSS)
    let processed_formatter = resolve_locale_specifiers(&processed_formatter, locale);
    let format = date_time.format_localized(&processed_formatter, locale);

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
    use chrono::Utc;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FormatDate)
    }

    /// #15266: `th_TH` carries `%Ey` in its `d_fmt`, so chrono failed to render
    /// `%x` under it and `nu` greeted the user with a type mismatch on startup.
    #[test]
    fn resolves_locale_specifiers_that_carry_an_alternative_modifier() {
        assert_eq!(
            resolve_locale_specifiers("%x %X", Locale::th_TH),
            "%d/%m/%y %H:%M:%S"
        );
        assert_eq!(
            resolve_locale_specifiers("%x %X", Locale::lo_LA),
            "%d/%m/%y %H:%M:%S"
        );
    }

    #[test]
    fn keeps_the_locale_field_order() {
        // The point of expanding rather than falling back to another locale:
        // day/month order survives. `en_US` would read `08/27/26` here.
        assert_eq!(resolve_locale_specifiers("%x", Locale::th_TH), "%d/%m/%y");
        assert_eq!(resolve_locale_specifiers("%x", Locale::en_US), "%m/%d/%Y");
        assert_eq!(
            resolve_locale_specifiers("%c", Locale::ja_JP),
            "%Y年%m月%d日 %H時%M分%S秒"
        );
    }

    #[test]
    fn strips_a_modifier_the_user_typed() {
        // POSIX: with no alternative representation available, behave as if the
        // modifier were absent. chrono has none, and used to error instead.
        assert_eq!(resolve_locale_specifiers("%Ey", Locale::en_US), "%y");
        assert_eq!(resolve_locale_specifiers("%Od", Locale::en_US), "%d");
    }

    /// `de_DE`, `fr_FR` and `nl_NL` have no am/pm time format, so `t_fmt_ampm`
    /// is "". Substituting the empty string for `%r` would erase the output;
    /// the plain time format is what chrono itself falls back to.
    #[test]
    fn falls_back_to_the_plain_time_format_when_the_locale_has_no_am_pm() {
        assert_eq!(resolve_locale_specifiers("%r", Locale::de_DE), "%T");
        assert_eq!(resolve_locale_specifiers("%r", Locale::fr_FR), "%T");
        assert_eq!(
            resolve_locale_specifiers("%r", Locale::en_US),
            "%I:%M:%S %p"
        );

        let dt = Utc
            .with_ymd_and_hms(2026, 8, 27, 13, 45, 0)
            .single()
            .expect("valid date");
        let span = Span::test_data();
        assert_eq!(
            format_from(dt, "%r", span, Locale::de_DE),
            Value::string("13:45:00", span),
            "an empty am/pm format must not blank the output"
        );
    }

    /// `az_IR` and `fa_IR` have both an empty `t_fmt_ampm` *and* a `t_fmt` that
    /// carries `%O`. Handing `%r` back to chrono let its own fallback pull that
    /// modifier in again, so those two still failed after #18918.
    #[test]
    fn resolves_the_am_pm_fallback_when_it_also_carries_a_modifier() {
        assert_eq!(resolve_locale_specifiers("%r", Locale::az_IR), "%H:%M:%S");
        assert_eq!(resolve_locale_specifiers("%r", Locale::fa_IR), "%H:%M:%S");

        let dt = Utc
            .with_ymd_and_hms(2026, 8, 27, 13, 45, 0)
            .single()
            .expect("valid date");
        let span = Span::test_data();
        assert_eq!(
            format_from(dt, "%r", span, Locale::az_IR),
            Value::string("13:45:00", span)
        );
    }

    /// Every locale whose `LC_TIME` strings carry `%E` or `%O` must render.
    #[test]
    fn renders_every_locale_that_carries_an_alternative_modifier() {
        let dt = Utc
            .with_ymd_and_hms(2026, 8, 27, 13, 45, 0)
            .single()
            .expect("valid date");
        let span = Span::test_data();

        for locale in [
            Locale::az_IR,
            Locale::fa_IR,
            Locale::lo_LA,
            Locale::lzh_TW,
            Locale::mnw_MM,
            Locale::my_MM,
            Locale::or_IN,
            Locale::shn_MM,
            Locale::th_TH,
        ] {
            for formatter in ["%x", "%X", "%c", "%r"] {
                let value = format_from(dt, formatter, span, locale);
                assert!(
                    matches!(value, Value::String { .. }),
                    "{locale:?} {formatter} did not render: {value:?}"
                );
            }
        }
    }

    #[test]
    fn leaves_literal_percent_and_unknown_specifiers_alone() {
        assert_eq!(resolve_locale_specifiers("%%x", Locale::th_TH), "%%x");
        assert_eq!(resolve_locale_specifiers("100%%", Locale::th_TH), "100%%");
        assert_eq!(
            resolve_locale_specifiers("%Y-%m-%d", Locale::th_TH),
            "%Y-%m-%d"
        );
        assert_eq!(resolve_locale_specifiers("", Locale::th_TH), "");
        assert_eq!(resolve_locale_specifiers("%", Locale::th_TH), "%");
    }

    /// The bug as the reporter met it: a real date, rendered under `th_TH`.
    #[test]
    fn renders_a_date_under_a_locale_that_used_to_fail() {
        let dt = Utc
            .with_ymd_and_hms(2026, 8, 27, 1, 45, 0)
            .single()
            .expect("valid date");
        let span = Span::test_data();

        let formatted = format_from(dt, "%x %X", span, Locale::th_TH);
        assert_eq!(
            formatted,
            Value::string("27/08/26 01:45:00", span),
            "th_TH must render, and must keep its own day/month order"
        );

        // A locale that never had the problem must be untouched.
        assert_eq!(
            format_from(dt, "%x %X", span, Locale::en_US),
            Value::string("08/27/2026 01:45:00 AM", span)
        );
    }
}
