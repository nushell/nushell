use crate::errors::ShellError;
use crate::object::{Dictionary, Value};
use crate::prelude::*;
use chrono::{DateTime, Local, Utc};

use crate::parser::registry::{CommandConfig, NamedType};
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;

pub struct Date;

impl Command for Date {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        date(args)
    }
    fn name(&self) -> &str {
        "date"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("utc".to_string(), NamedType::Switch);
        named.insert("local".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: false,
            named,
            is_sink: true,
            is_filter: false,
        }
    }
}

pub fn date_to_value<T: TimeZone>(dt: DateTime<T>, span: Span) -> Tagged<Value>
where
    T::Offset: Display,
{
    let mut indexmap = IndexMap::new();

    indexmap.insert(
        "year".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.year()), span),
    );
    indexmap.insert(
        "month".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.month()), span),
    );
    indexmap.insert(
        "day".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.day()), span),
    );
    indexmap.insert(
        "hour".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.hour()), span),
    );
    indexmap.insert(
        "minute".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.minute()), span),
    );
    indexmap.insert(
        "second".to_string(),
        Tagged::from_simple_spanned_item(Value::int(dt.second()), span),
    );

    let tz = dt.offset();
    indexmap.insert(
        "timezone".to_string(),
        Tagged::from_simple_spanned_item(Value::string(format!("{}", tz)), span),
    );

    Tagged::from_simple_spanned_item(Value::Object(Dictionary::from(indexmap)), span)
}

pub fn date(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut date_out = VecDeque::new();
    let span = args.call_info.name_span;

    let value = if args.has("utc") {
        let utc: DateTime<Utc> = Utc::now();
        date_to_value(utc, span)
    } else {
        let local: DateTime<Local> = Local::now();
        date_to_value(local, span)
    };

    date_out.push_back(value);

    Ok(date_out.to_output_stream())
}
