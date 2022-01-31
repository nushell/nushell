use chrono::{DateTime, FixedOffset, Local, LocalResult, Offset, TimeZone, Utc};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

struct Arguments {
    timezone: Option<Spanned<String>>,
    offset: Option<Spanned<i64>>,
    format: Option<String>,
    column_paths: Vec<CellPath>,
}

// In case it may be confused with chrono::TimeZone
#[derive(Clone, Debug)]
enum Zone {
    Utc,
    Local,
    East(u8),
    West(u8),
    Error, // we want the nullshell to cast it instead of rust
}

impl Zone {
    fn new(i: i64) -> Self {
        if i.abs() <= 12 {
            // guanranteed here
            if i >= 0 {
                Self::East(i as u8) // won't go out of range
            } else {
                Self::West(-i as u8) // same here
            }
        } else {
            Self::Error // Out of range
        }
    }
    fn from_string(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "utc" | "u" => Self::Utc,
            "local" | "l" => Self::Local,
            _ => Self::Error,
        }
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into datetime"
    }

    fn signature(&self) -> Signature {
        Signature::build("into datetime")
            .switch(
                "list",
                "lists strftime cheatsheet",
                Some('l'),
                )
            .named(
                "timezone",
                SyntaxShape::String,
                "Specify timezone if the input is timestamp, like 'UTC/u' or 'LOCAL/l'",
                Some('z'),
            )
            .named(
                "offset",
                SyntaxShape::Int,
                "Specify timezone by offset if the input is timestamp, like '+8', '-4', prior than timezone",
                Some('o'),
            )
            .named(
                "format",
                SyntaxShape::String,
                "Specify date and time formatting",
                Some('f'),
            )
            .rest(
            "rest",
                SyntaxShape::CellPath,
                "optionally convert text into datetime by column paths",
            )
            .category(Category::Conversions)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn usage(&self) -> &str {
        "converts text into datetime"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to datetime",
                example: "'16.11.1984 8:00 am +0000' | into datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime",
                example: "'2020-08-04T16:39:18+00:00' | into datetime",
                result: None,
            },
            Example {
                description: "Convert to datetime using a custom format",
                example: "'20200904_163918+0000' | into datetime -f '%Y%m%d_%H%M%S%z'",
                result: None,
            },
            Example {
                description: "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone",
                example: "'1614434140' | into datetime -z 'UTC'",
                result: None,
            },
            Example {
                description:
                    "Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)",
                example: "'1614434140' | into datetime -o +9",
                result: None,
            },
        ]
    }
}

#[derive(Clone)]
struct DatetimeFormat(String);

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;

    let options = Arguments {
        timezone: call.get_flag(engine_state, stack, "timezone")?,
        offset: call.get_flag(engine_state, stack, "offset")?,
        format: call.get_flag(engine_state, stack, "format")?,
        column_paths: call.rest(engine_state, stack, 0)?,
    };

    // if zone-offset is specified, then zone will be neglected
    let zone_options = match &options.offset {
        Some(zone_offset) => Some(Spanned {
            item: Zone::new(zone_offset.item),
            span: zone_offset.span,
        }),
        None => options.timezone.as_ref().map(|zone| Spanned {
            item: Zone::from_string(zone.item.clone()),
            span: zone.span,
        }),
    };

    let list_flag = call.has_flag("list");

    let format_options = options
        .format
        .as_ref()
        .map(|fmt| DatetimeFormat(fmt.to_string()));

    input.map(
        move |v| {
            if options.column_paths.is_empty() && !list_flag {
                action(&v, &zone_options, &format_options, head)
            } else if list_flag {
                generate_strfttime_list(head)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let zone_options = zone_options.clone();
                    let format_options = format_options.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &zone_options, &format_options, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn generate_strfttime_list(head: Span) -> Value {
    let column_names = vec![
        "Specification".into(),
        "Example".into(),
        "Description".into(),
    ];
    let records = vec![
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%Y".into(),
                    span: head,
                },
                Value::String {
                    val: "2001".into(),
                    span: head,
                },
                Value::String {
                    val: "The full proleptic Gregorian year, zero-padded to 4 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%C".into(),
                    span: head,
                },
                Value::String {
                    val: "20".into(),
                    span: head,
                },
                Value::String {
                    val: "The proleptic Gregorian year divided by 100, zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%y".into(),
                    span: head,
                },
                Value::String {
                    val: "01".into(),
                    span: head,
                },
                Value::String {
                    val: "The proleptic Gregorian year modulo 100, zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%m".into(),
                    span: head,
                },
                Value::String {
                    val: "07".into(),
                    span: head,
                },
                Value::String {
                    val: "Month number (01--12), zero-padded to 2 digits.".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%b".into(),
                    span: head,
                },
                Value::String {
                    val: "Jul".into(),
                    span: head,
                },
                Value::String {
                    val: "Abbreviated month name. Always 3 letters".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%B".into(),
                    span: head,
                },
                Value::String {
                    val: "July".into(),
                    span: head,
                },
                Value::String {
                    val: "Full month name. Also accepts corresponding abbreviation in parsing"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%h".into(),
                    span: head,
                },
                Value::String {
                    val: "Jul".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %b".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%d".into(),
                    span: head,
                },
                Value::String {
                    val: "08".into(),
                    span: head,
                },
                Value::String {
                    val: "Day number (01--31), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%e".into(),
                    span: head,
                },
                Value::String {
                    val: "8".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %d but space-padded. Same to %_d".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%a".into(),
                    span: head,
                },
                Value::String {
                    val: "Sun".into(),
                    span: head,
                },
                Value::String {
                    val: "Abbreviated weekday name. Always 3 letters".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%A".into(),
                    span: head,
                },
                Value::String {
                    val: "Sunday".into(),
                    span: head,
                },
                Value::String {
                    val: "Full weekday name. Also accepts corresponding abbreviation in parsing"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%w".into(),
                    span: head,
                },
                Value::String {
                    val: "0".into(),
                    span: head,
                },
                Value::String {
                    val: "Sunday = 0, Monday = 1, ..., Saturday = 6".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%u".into(),
                    span: head,
                },
                Value::String {
                    val: "7".into(),
                    span: head,
                },
                Value::String {
                    val: "Monday = 1, Tuesday = 2, ..., Sunday = 7. (ISO 8601)
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%U".into(),
                    span: head,
                },
                Value::String {
                    val: "28".into(),
                    span: head,
                },
                Value::String {
                    val: "Week number starting with Sunday (00--53), zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%W".into(),
                    span: head,
                },
                Value::String {
                    val: "27".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %U, but week 1 starts with the first Monday in that year instead"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%G".into(),
                    span: head,
                },
                Value::String {
                    val: "2001".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %Y but uses the year number in ISO 8601 week date. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%g".into(),
                    span: head,
                },
                Value::String {
                    val: "01".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %y but uses the year number in ISO 8601 week date. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%V".into(),
                    span: head,
                },
                Value::String {
                    val: "27".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %U but uses the week number in ISO 8601 week date (01--53). 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%j".into(),
                    span: head,
                },
                Value::String {
                    val: "189".into(),
                    span: head,
                },
                Value::String {
                    val: "Day of the year (001--366), zero-padded to 3 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%D".into(),
                    span: head,
                },
                Value::String {
                    val: "07/08/01".into(),
                    span: head,
                },
                Value::String {
                    val: "Month-day-year format. Same to %m/%d/%y".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%x".into(),
                    span: head,
                },
                Value::String {
                    val: "07/08/01".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %D".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%F".into(),
                    span: head,
                },
                Value::String {
                    val: "2001-07-08".into(),
                    span: head,
                },
                Value::String {
                    val: "Year-month-day format (ISO 8601). Same to %Y-%m-%d".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%v".into(),
                    span: head,
                },
                Value::String {
                    val: "8-Jul-2001".into(),
                    span: head,
                },
                Value::String {
                    val: "Day-month-year format. Same to %e-%b-%Y".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%H".into(),
                    span: head,
                },
                Value::String {
                    val: "00".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour number (00--23), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%k".into(),
                    span: head,
                },
                Value::String {
                    val: "0".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %H but space-padded. Same to %_H".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%I".into(),
                    span: head,
                },
                Value::String {
                    val: "12".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour number in 12-hour clocks (01--12), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%l".into(),
                    span: head,
                },
                Value::String {
                    val: "12".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %I but space-padded. Same to %_I".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%P".into(),
                    span: head,
                },
                Value::String {
                    val: "am".into(),
                    span: head,
                },
                Value::String {
                    val: "am or pm in 12-hour clocks".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%p".into(),
                    span: head,
                },
                Value::String {
                    val: "AM".into(),
                    span: head,
                },
                Value::String {
                    val: "AM or PM in 12-hour clocks".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%M".into(),
                    span: head,
                },
                Value::String {
                    val: "34".into(),
                    span: head,
                },
                Value::String {
                    val: "Minute number (00--59), zero-padded to 2 digits".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%S".into(),
                    span: head,
                },
                Value::String {
                    val: "60".into(),
                    span: head,
                },
                Value::String {
                    val: "Second number (00--60), zero-padded to 2 digits. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%f".into(),
                    span: head,
                },
                Value::String {
                    val: "026490000".into(),
                    span: head,
                },
                Value::String {
                    val: "The fractional seconds (in nanoseconds) since last whole second. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 3. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 6. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%.".into(),
                    span: head,
                },
                Value::String {
                    val: ".026490000".into(),
                    span: head,
                },
                Value::String {
                    val: "Similar to .%f but left-aligned but fixed to a length of 9. 
"
                    .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%R".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour-minute format. Same to %H:%M".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%T".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "Hour-minute-second format. Same to %H:%M:%S".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%X".into(),
                    span: head,
                },
                Value::String {
                    val: "00:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %T".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%r".into(),
                    span: head,
                },
                Value::String {
                    val: "12:34:60".into(),
                    span: head,
                },
                Value::String {
                    val: "AM Hour-minute-second format in 12-hour clocks. Same to %I:%M:%S %p"
                        .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%Z".into(),
                    span: head,
                },
                Value::String {
                    val: "ACST".into(),
                    span: head,
                },
                Value::String {
                    val: "Formatting only: Local time zone name".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%z".into(),
                    span: head,
                },
                Value::String {
                    val: "+0930".into(),
                    span: head,
                },
                Value::String {
                    val: "Offset from the local time to UTC (with UTC being +0000)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%:".into(),
                    span: head,
                },
                Value::String {
                    val: "+09:30".into(),
                    span: head,
                },
                Value::String {
                    val: "Same to %z but with a colon".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%c".into(),
                    span: head,
                },
                Value::String {
                    val: "Sun".into(),
                    span: head,
                },
                Value::String {
                    val:
                        "Jul 8 00:34:60 2001 ctime date & time format. Same to %a %b %e %T %Y sans"
                            .into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%s".into(),
                    span: head,
                },
                Value::String {
                    val: "994518299".into(),
                    span: head,
                },
                Value::String {
                    val: "UNIX timestamp, the number of seconds since 1970-01-01 00:00 UTC.".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%t".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "Literal tab (\\t)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names.clone(),
            vals: vec![
                Value::String {
                    val: "%n".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "Literal newline (\\n)".into(),
                    span: head,
                },
            ],
            span: head,
        },
        Value::Record {
            cols: column_names,
            vals: vec![
                Value::String {
                    val: "%%".into(),
                    span: head,
                },
                Value::String {
                    val: "".into(),
                    span: head,
                },
                Value::String {
                    val: "percent sign".into(),
                    span: head,
                },
            ],
            span: head,
        },
    ];

    Value::List {
        vals: records,
        span: head,
    }
}

fn action(
    input: &Value,
    timezone: &Option<Spanned<Zone>>,
    dateformat: &Option<DatetimeFormat>,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, span, .. } => {
            let ts = s.parse::<i64>();
            // if timezone if specified, first check if the input is a timestamp.
            if let Some(tz) = timezone {
                const TIMESTAMP_BOUND: i64 = 8.2e+12 as i64;
                // Since the timestamp method of chrono itself don't throw an error (it just panicked)
                // We have to manually guard it.
                if let Ok(t) = ts {
                    if t.abs() > TIMESTAMP_BOUND {
                        return Value::Error{error: ShellError::UnsupportedInput(
                            "Given timestamp is out of range, it should between -8e+12 and 8e+12".to_string(),
                            head,
                        )};
                    }
                    const HOUR: i32 = 3600;
                    let stampout = match tz.item {
                        Zone::Utc => Value::Date {
                            val: Utc.timestamp(t, 0).into(),
                            span: head,
                        },
                        Zone::Local => Value::Date {
                            val: Local.timestamp(t, 0).into(),
                            span: head,
                        },
                        Zone::East(i) => {
                            let eastoffset = FixedOffset::east((i as i32) * HOUR);
                            Value::Date {
                                val: eastoffset.timestamp(t, 0),
                                span: head,
                            }
                        }
                        Zone::West(i) => {
                            let westoffset = FixedOffset::west((i as i32) * HOUR);
                            Value::Date {
                                val: westoffset.timestamp(t, 0),
                                span: head,
                            }
                        }
                        Zone::Error => Value::Error {
                            error: ShellError::UnsupportedInput(
                                "Cannot convert given timezone or offset to timestamp".to_string(),
                                tz.span,
                            ),
                        },
                    };
                    return stampout;
                }
            };
            // if it's not, continue and negelect the timezone option.
            let out = match dateformat {
                Some(dt) => match DateTime::parse_from_str(s, &dt.0) {
                    Ok(d) => Value::Date { val: d, span: head },
                    Err(reason) => {
                        return Value::Error {
                            error: ShellError::CantConvert(
                                format!("could not parse as datetime using format '{}'", dt.0),
                                reason.to_string(),
                                head,
                            ),
                        }
                    }
                },
                None => match dtparse::parse(s) {
                    Ok((native_dt, fixed_offset)) => {
                        let offset = match fixed_offset {
                            Some(fo) => fo,
                            None => FixedOffset::east(0).fix(),
                        };
                        match offset.from_local_datetime(&native_dt) {
                            LocalResult::Single(d) => Value::Date { val: d, span: head },
                            LocalResult::Ambiguous(d, _) => Value::Date { val: d, span: head },
                            LocalResult::None => {
                                return Value::Error {
                                    error: ShellError::CantConvert(
                                        "could not convert to a timezone-aware datetime"
                                            .to_string(),
                                        "local time representation is invalid".to_string(),
                                        head,
                                    ),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        return Value::Error {
                            error: ShellError::UnsupportedInput(
                                "Cannot convert input string as datetime. Might be missing timezone or offset".to_string(),
                                *span,
                            ),
                        }
                    }
                },
            };

            out
        }
        other => {
            let got = format!("Expected string, got {} instead", other.get_type());
            Value::Error {
                error: ShellError::UnsupportedInput(got, head),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{action, DatetimeFormat, SubCommand, Zone};
    use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn takes_a_date_format() {
        let date_str = Value::test_string("16.11.1984 8:00 am +0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("16.11.1984 8:00 am +0000", "%d.%m.%Y %H:%M %P %z")
                .unwrap(),
            span: Span::test_data(),
        };
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_iso8601_date_format() {
        let date_str = Value::test_string("2020-08-04T16:39:18+00:00");
        let actual = action(&date_str, &None, &None, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("2020-08-04T16:39:18+00:00", "%Y-%m-%dT%H:%M:%S%z")
                .unwrap(),
            span: Span::test_data(),
        };
        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp_offset() {
        let date_str = Value::test_string("1614434140");
        let timezone_option = Some(Spanned {
            item: Zone::East(8),
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date {
            val: DateTime::parse_from_str("2021-02-27 21:55:40 +08:00", "%Y-%m-%d %H:%M:%S %z")
                .unwrap(),
            span: Span::test_data(),
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_timestamp() {
        let date_str = Value::test_string("1614434140");
        let timezone_option = Some(Spanned {
            item: Zone::Local,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());
        let expected = Value::Date {
            val: Local.timestamp(1614434140, 0).into(),
            span: Span::test_data(),
        };

        assert_eq!(actual, expected)
    }

    #[test]
    fn takes_invalid_timestamp() {
        let date_str = Value::test_string("10440970000000");
        let timezone_option = Some(Spanned {
            item: Zone::Utc,
            span: Span::test_data(),
        });
        let actual = action(&date_str, &timezone_option, &None, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_datetimelike_string() {
        let date_str = Value::test_string("16.11.1984 8:00 am Oops0000");
        let fmt_options = Some(DatetimeFormat("%d.%m.%Y %H:%M %P %z".to_string()));
        let actual = action(&date_str, &None, &fmt_options, Span::test_data());

        assert_eq!(actual.get_type(), Error);
    }
}
