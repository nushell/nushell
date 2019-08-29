use crate::errors::ShellError;
use crate::object::{Dictionary, Value};
use crate::prelude::*;
use chrono::{DateTime, Local, Utc};

use crate::commands::WholeStreamCommand;
use crate::parser::registry::Signature;
use chrono::{Datelike, TimeZone, Timelike};
use core::fmt::Display;
use indexmap::IndexMap;

pub struct Date;

impl WholeStreamCommand for Date {
    fn name(&self) -> &str {
        "date"
    }

    fn signature(&self) -> Signature {
        Signature::build("date")
            .switch("utc")
            .switch("local")
    }

    fn usage(&self) -> &str {
        "Get the current datetime."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        date(args, registry)
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

pub fn date(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

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
