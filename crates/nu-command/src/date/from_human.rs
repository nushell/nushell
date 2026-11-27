use super::formats::{parse_gnu_date, parse_relative};
use fancy_regex::Regex;
use jiff::{ZonedDateTime, civil};
use nu_engine::command_prelude::*;
use nu_protocol::DateTime;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct DateFromHuman;

impl Command for DateFromHuman {
    fn name(&self) -> &str {
        "date from-human"
    }

    fn signature(&self) -> Signature {
        Signature::build("date from-human")
            .input_output_types(vec![
                (Type::String, Type::Date),
                (Type::Nothing, Type::table()),
                (
                    Type::List(Box::new(Type::Date)),
                    Type::List(Box::new(Type::Date)),
                ),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Date)),
                ),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "list",
                "Show human-readable datetime parsing examples.",
                Some('l'),
            )
            .category(Category::Date)
    }

    fn description(&self) -> &str {
        "Convert a human readable datetime string to a datetime."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "relative",
            "now",
            "today",
            "tomorrow",
            "yesterday",
            "weekday",
            "weekday_name",
            "timezone",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if call.has_flag(engine_state, stack, "list")? {
            return Ok(list_human_readable_examples(call.head).into_pipeline_data());
        }
        let head = call.head;
        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(move |value| helper(value, head), engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Parsing human readable datetime.",
                example: "'Today at 18:30' | date from-human",
                result: None,
            },
            Example {
                description: "Parsing human readable datetime.",
                example: "'Last Friday at 19:45' | date from-human",
                result: None,
            },
            Example {
                description: "Parsing human readable datetime.",
                example: "'In 5 minutes and 30 seconds' | date from-human",
                result: None,
            },
            Example {
                description: "Show human-readable datetime parsing examples.",
                example: "date from-human --list",
                result: None,
            },
            Example {
                description: "Convert a list of human-readable datetime strings to datetimes.",
                example: r#"["Today at 18:30", "Tomorrow at 09:00"] | date from-human"#,
                result: None,
            },
        ]
    }
}

fn helper(value: Value, head: Span) -> Value {
    let span = value.span();
    let input_val = match value {
        Value::String { val, .. } => val,
        other => {
            return Value::error(
                ShellError::OnlySupportsThisInputType {
                    exp_input_type: "string".to_string(),
                    wrong_type: other.get_type().to_string(),
                    dst_span: head,
                    src_span: span,
                },
                span,
            );
        }
    };

    match read(&input_val, &ZonedDateTime::now()) {
        Some(date) => Value::date(date, span),
        None => Value::error(
            ShellError::IncorrectValue {
                msg: "Cannot parse as humanized date".to_string(),
                val_span: head,
                call_span: span,
            },
            span,
        ),
    }
}

/// The date `text` names in GNU `date` syntax, after [`to_gnu_date`]'s rewrites (`today at
/// 6pm`, `in 3 days`, `next friday`, `2022-11-07`). GNU syntax reads the text only with a time
/// of day, a date or a month name, or (as in `into datetime`) a relative word, `@` or `TZ=`, so
/// a lone zone letter, zone name or number (`x`, `est`, `12`) is no date. Without the `os`
/// feature, which brings GNU syntax, the text is read as `into datetime` reads it.
fn read(text: &str, now: &ZonedDateTime) -> Option<DateTime> {
    let gnu = to_gnu_date(text);
    let has = |pattern: &Regex| pattern.is_match(text).unwrap_or(false);
    let date = if has(&TIME_OF_DAY) || has(&DATE) {
        parse_gnu_date(&gnu)
    } else {
        parse_relative(&gnu, None)
    };
    #[cfg(not(feature = "os"))]
    let date = date.or_else(|| super::formats::parse_date_text(text, None));
    // A date alone takes the time of day in its `TZ="..."` zone.
    match split_zone(text).0 {
        Some(zone) => with_time_of_day(date?, text, &now.in_tz(zone).ok()?),
        None => with_time_of_day(date?, text, now),
    }
}

/// Matches a whole date with its year (`2022-11-07`, `2022/11/07`, `11/07/2022`) or a month
/// name. Shorter number pairs (`1/2`, `10-3`) are fractions, ranges and versions as often as dates.
static DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)\b\d{4}[-/.]\d{1,2}[-/.]\d{1,2}\b|\b\d{1,2}[-/.]\d{1,2}[-/.]\d{2,4}\b",
        r"|\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\b",
    ))
    .expect("date pattern should be valid")
});

/// Matches the parts of a phrase that name a time of day or a span shorter than a day.
static TIME_OF_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\d:\d|\d\s*[ap]\.?m\b|\b(noon|midnight|now|hours?|hrs?|minutes?|mins?|seconds?|secs?)\b",
    )
    .expect("time of day pattern should be valid")
});

/// A reading of a phrase that names no time of day (`Friday`, `2022-11-07`) is at the current
/// time of day, as human-date-parser, which `date from-human` used before, read it. GNU reads
/// such a phrase as midnight, so a reading at any other time (`today 1430`) keeps its time.
fn with_time_of_day(date: DateTime, text: &str, now: &ZonedDateTime) -> Option<DateTime> {
    if TIME_OF_DAY.is_match(text).unwrap_or(false) {
        return Some(date);
    }
    let wall = date.to_civil();
    if wall.time() != civil::Time::midnight() {
        return Some(date);
    }
    let zoned = wall
        .date()
        .to_datetime(now.time())
        .to_zoned_datetime(now.time_zone().clone())
        .ok()?;
    Some(DateTime::from_zoned(&zoned))
}

/// Rewrites the English the GNU `date` grammar doesn't read: `at` and `and` go, `in 2 hours`
/// becomes `2 hours`, `a day ago` becomes `1 day ago`, `overmorrow` is `2 days`, `upcoming` is
/// `next`, `3d`, `2h`, `5m` and `30s` spell out their units, and a final `ago` applies to every
/// unit before it (GNU's applies to one), so `10 hours and 5 minutes ago` is `10 hours ago 5
/// minutes ago`. A leading `TZ="..."` stays as written, since GNU reads only an upper-case `TZ`
/// and zone names have case (`Europe/Paris`).
fn to_gnu_date(text: &str) -> String {
    let (zone, text) = split_zone(text);
    let rest = to_gnu_words(text);
    match zone {
        Some(zone) => format!("TZ=\"{zone}\" {rest}"),
        None => rest,
    }
}

/// The zone name of a leading `TZ="..."` (in any case) and the text after it.
fn split_zone(text: &str) -> (Option<&str>, &str) {
    let text = text.trim_start();
    let quote = text
        .get(..4)
        .filter(|start| start.eq_ignore_ascii_case("tz=\""))
        .and_then(|_| text[4..].find('"'));
    match quote {
        Some(quote) => (Some(&text[4..4 + quote]), &text[5 + quote..]),
        None => (None, text),
    }
}

/// [`to_gnu_date`]'s rewrites of the words.
fn to_gnu_words(text: &str) -> String {
    let lower = text.to_lowercase().replace(',', " ");
    let mut words: Vec<&str> = lower.split_whitespace().flat_map(expand_word).collect();
    let all_ago = words.last() == Some(&"ago");
    if all_ago {
        words.pop();
    }
    let is_count = |word: Option<&&str>| {
        word.is_some_and(|word| matches!(*word, "a" | "an") || word.parse::<u64>().is_ok())
    };
    let mut out = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        match *word {
            "at" | "and" => {}
            "in" if is_count(words.get(i + 1)) => {}
            "a" | "an"
                if words
                    .get(i + 1)
                    .is_some_and(|next| next.ends_with(|c: char| c.is_alphabetic())) =>
            {
                out.push("1");
            }
            word => {
                out.push(word);
                if all_ago && is_unit(word) {
                    out.push("ago");
                }
            }
        }
    }
    out.join(" ")
}

/// `word` in words GNU `date` reads: `overmorrow` is `2 days`, `upcoming` is `next`, and a
/// number with one of the unit letters `d`, `h`, `m` or `s` is the number and the unit.
fn expand_word(word: &str) -> Vec<&str> {
    let unit = match word.as_bytes().split_last() {
        Some((last, digits)) if !digits.is_empty() && digits.iter().all(u8::is_ascii_digit) => {
            match last {
                b'd' => Some("days"),
                b'h' => Some("hours"),
                b'm' => Some("minutes"),
                b's' => Some("seconds"),
                _ => None,
            }
        }
        _ => None,
    };
    match (word, unit) {
        ("overmorrow", _) => vec!["2", "days"],
        ("upcoming", _) => vec!["next"],
        (_, Some(unit)) => vec![&word[..word.len() - 1], unit],
        _ => vec![word],
    }
}

/// Whether `word` is a GNU `date` relative unit, such as `hours` or `min`.
fn is_unit(word: &str) -> bool {
    matches!(
        word.trim_end_matches('s'),
        "year"
            | "month"
            | "fortnight"
            | "week"
            | "day"
            | "hour"
            | "minute"
            | "min"
            | "second"
            | "sec"
    )
}

fn list_human_readable_examples(span: Span) -> Value {
    let examples: Vec<String> = vec![
        "Today 18:30".into(),
        "2022-11-07 13:25:30".into(),
        "15:20 Friday".into(),
        "This Friday 17:00".into(),
        "13:25, Next Tuesday".into(),
        "Last Friday at 19:45".into(),
        "In 3 days".into(),
        "In 2 hours".into(),
        "10 hours and 5 minutes ago".into(),
        "1 years ago".into(),
        "A year ago".into(),
        "A month ago".into(),
        "A week ago".into(),
        "A day ago".into(),
        "An hour ago".into(),
        "A minute ago".into(),
        "A second ago".into(),
        "Now".into(),
    ];

    let records = examples
        .iter()
        .map(|s| {
            Value::record(
                record! {
                    "parseable human datetime examples" => Value::test_string(s.to_string()),
                    "result" => helper(Value::test_string(s.to_string()), span),
                },
                span,
            )
        })
        .collect::<Vec<Value>>();

    Value::list(records, span)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(DateFromHuman)
    }

    #[cfg(feature = "os")]
    #[test]
    fn phrases_read_in_gnu_syntax() {
        let now = ZonedDateTime::now();
        let day = |text: &str| read(text, &now).map(|date| date.to_civil().date());
        let today = now.date();
        for (text, days) in [
            ("today at 6pm", 0),
            ("tomorrow", 1),
            ("overmorrow", 2),
            ("in 3d", 3),
            ("In 3 days", 3),
            ("a day ago", -1),
        ] {
            let expected = today.checked_add(jiff::Span::new().days(days)).ok();
            assert_eq!(day(text), expected, "{text}");
        }
        let hour = |text: &str| read(text, &now).map(|date| date.parts().hour);
        assert_eq!(hour("today at 6pm"), Some(18));
        assert_eq!(hour("Last Friday at 19:45"), Some(19));
        assert_eq!(
            read("upcoming friday", &now).map(|date| date.to_civil().weekday()),
            Some(jiff::civil::Weekday::Friday)
        );
        // A date alone keeps the current time of day.
        let date = read("2022-11-07", &now).expect("a date");
        assert_eq!(date.to_civil().date(), jiff::civil::date(2022, 11, 7));
        // Text GNU syntax doesn't read, or that is no date without a relative word, `@`, a date
        // or a time of day: military zone letters, zone abbreviations, bare numbers.
        for text in [
            "blah",
            "today blah",
            "5 apples",
            "a b c",
            "utc time please",
            "10 apples ago",
            "x",
            "a",
            "z",
            "est",
            "12",
            "2025",
            "1430",
            "3.14",
            "1/2",
            "10-3",
            "1.2.3",
            "192.168.1.1",
        ] {
            assert!(read(text, &now).is_none(), "{text}");
        }
    }

    #[cfg(feature = "os")]
    #[test]
    fn gnu_syntax_without_a_phrase_needs_a_relative_word_or_a_time() {
        let now = ZonedDateTime::now();
        let hour = |text: &str| read(text, &now).map(|date| date.parts().hour);
        for text in ["6pm tomorrow", "6pm", "18:00 tomorrow"] {
            assert_eq!(hour(text), Some(18), "{text}");
        }
        assert_eq!(
            read("@1744972472", &now).map(DateTime::unix_seconds),
            Some(1_744_972_472)
        );
    }

    #[cfg(feature = "os")]
    #[test]
    fn gnu_time_without_a_colon_is_kept() {
        let now = ZonedDateTime::now();
        let time = |text: &str| read(text, &now).map(|date| date.parts());
        let parts = time("today 1430").expect("today 1430 is a date");
        assert_eq!((parts.hour, parts.minute, parts.second), (14, 30, 0));
    }

    /// A leading `TZ="..."` keeps its case, in any case of `TZ`, and a date alone takes the
    /// current time of day in that zone.
    #[cfg(feature = "os")]
    #[test]
    fn zone_prefix() {
        let now = ZonedDateTime::now();
        let read = |text: &str| read(text, &now).map(DateTime::to_rfc3339);
        assert_eq!(
            read(r#"TZ="Europe/Paris" 2025-04-18 04:04"#).as_deref(),
            Some("2025-04-18T04:04:00+02:00")
        );
        assert_eq!(
            read(r#"tz="Asia/Tokyo" 2025-04-18 04:04"#).as_deref(),
            Some("2025-04-18T04:04:00+09:00")
        );
        let date = super::read(r#"TZ="Asia/Tokyo" 2025-04-18"#, &now).expect("a date");
        assert_eq!(date.offset().seconds(), 9 * 3600);
        assert_eq!(date.to_civil().date(), jiff::civil::date(2025, 4, 18));
    }
}
