use super::{formats::parse_gnu_date, human::Phrase};
use fancy_regex::Regex;
use jiff::ZonedDateTime;
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

    match read_start(&input_val, &ZonedDateTime::now()) {
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

/// The date `text` names. Text after one of human-date-parser's phrases is ignored, as
/// human-date-parser ignored it: the longest start of the text that is a phrase names the date,
/// unless a longer start that ends before a space reads as GNU syntax, which keeps the time in
/// `today at 6pm blah`. Text that doesn't start with a phrase is no date.
fn read_start(text: &str, now: &ZonedDateTime) -> Option<DateTime> {
    if let Some(date) = read(text, now) {
        return date;
    }
    let starts = Phrase::starts(text);
    let (at, phrase) = starts
        .iter()
        .enumerate()
        .find_map(|(i, start)| Some((i, Phrase::parse(start)?)))?;
    starts[..at]
        .iter()
        .filter(|start| text[start.len()..].starts_with(' '))
        .find_map(|start| read(start, now))
        .unwrap_or_else(|| phrase.date(now))
}

/// The date `text` names, as one of human-date-parser's phrases or else in GNU `date` syntax.
/// `Some(None)` is a phrase that names no date, such as `2022-02-30`.
fn read(text: &str, now: &ZonedDateTime) -> Option<Option<DateTime>> {
    if let Some(phrase) = Phrase::parse(text) {
        return Some(phrase.date(now));
    }
    let date = parse_gnu_date(&to_gnu_date(text))?;
    Some(with_time_of_day(date, text, now))
}

/// Matches the parts of a phrase that name a time of day or a span shorter than a day.
static TIME_OF_DAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\d:\d|\d\s*[ap]\.?m\b|\b(noon|midnight|now|hours?|hrs?|minutes?|mins?|seconds?|secs?)\b",
    )
    .expect("time of day pattern should be valid")
});

/// A GNU `date` reading of a phrase that names no time of day (`Friday`, `2022-11-07`) is at the
/// current time of day, as a human-date-parser phrase that names a day is.
fn with_time_of_day(date: DateTime, text: &str, now: &ZonedDateTime) -> Option<DateTime> {
    if TIME_OF_DAY.is_match(text).unwrap_or(false) {
        return Some(date);
    }
    let wall = date.to_civil()?.date().to_datetime(now.time());
    let zoned = wall.to_zoned_datetime(now.time_zone().clone()).ok()?;
    Some(DateTime::from_zoned(&zoned))
}

/// Rewrites the English the GNU `date` grammar doesn't read: `at` and `and` go, `in 2 hours`
/// becomes `2 hours`, `a day ago` becomes `1 day ago`, and a final `ago` applies to every unit
/// before it (GNU's applies to one), so `10 hours and 5 minutes ago` is `10 hours ago 5 minutes
/// ago`.
fn to_gnu_date(text: &str) -> String {
    let lower = text.to_lowercase().replace(',', " ");
    let mut words: Vec<&str> = lower.split_whitespace().collect();
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
    fn text_after_a_phrase_keeps_its_time() {
        let now = ZonedDateTime::now();
        let hour = |text: &str| read_start(text, &now).map(|date| date.parts().hour);
        // `6pm` is GNU syntax; human-date-parser's grammar reads `18:00`.
        for text in ["today at 6pm", "today at 6pm blah", "today at 18:00 blah"] {
            assert_eq!(hour(text), Some(18), "{text}");
        }
        // Text that doesn't start with a phrase is no date, even when GNU syntax reads a word
        // of it.
        for text in [
            "blah",
            "5 apples",
            "a b c",
            "utc time please",
            "10 apples ago",
        ] {
            assert!(read_start(text, &now).is_none(), "{text}");
        }
    }
}
