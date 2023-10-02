use ical::parser::ical::component::*;
use ical::property::Property;
use indexmap::map::IndexMap;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{record, PluginExample, Record, ShellError, Span, Value};
use std::io::BufReader;

pub const CMD_NAME: &str = "from ics";

pub fn from_ics_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let span = input.span();
    let input_string = input.as_string()?;
    let head = call.head;

    let mut input_v: Vec<String> = Vec::new();
    for line in input_string.lines() {
        if line.starts_with(' ') {
            match input_v.pop() {
                None => {
                    input_v.push(line.trim().to_string());
                }
                Some(prev_line) => {
                    input_v.push(prev_line + line.trim());
                }
            }
        } else {
            input_v.push(line.trim().to_string());
        }
    }
    let input_string = input_v.join("\n");

    let input_bytes = input_string.as_bytes();
    let buf_reader = BufReader::new(input_bytes);
    let parser = ical::IcalParser::new(buf_reader);

    let mut output = vec![];

    for calendar in parser {
        match calendar {
            Ok(c) => output.push(calendar_to_value(c, head)),
            Err(e) => output.push(Value::error(
                ShellError::UnsupportedInput(
                    format!("input cannot be parsed as .ics ({e})"),
                    "value originates from here".into(),
                    head,
                    span,
                ),
                span,
            )),
        }
    }
    Ok(Value::list(output, head))
}

pub fn examples() -> Vec<PluginExample> {
    vec![PluginExample {
        example: "'BEGIN:VCALENDAR
            END:VCALENDAR' | from ics"
            .into(),
        description: "Converts ics formatted string to table".into(),
        result: Some(Value::list(
            vec![Value::test_record(Record {
                cols: vec![
                    "properties".to_string(),
                    "events".to_string(),
                    "alarms".to_string(),
                    "to-Dos".to_string(),
                    "journals".to_string(),
                    "free-busys".to_string(),
                    "timezones".to_string(),
                ],
                vals: vec![
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                    Value::list(vec![], Span::test_data()),
                ],
            })],
            Span::test_data(),
        )),
    }]
}

fn calendar_to_value(calendar: IcalCalendar, span: Span) -> Value {
    Value::record(
        record! {
            "properties" => properties_to_value(calendar.properties, span),
            "events" => events_to_value(calendar.events, span),
            "alarms" => alarms_to_value(calendar.alarms, span),
            "to-Dos" => todos_to_value(calendar.todos, span),
            "journals" => journals_to_value(calendar.journals, span),
            "free-busys" => free_busys_to_value(calendar.free_busys, span),
            "timezones" => timezones_to_value(calendar.timezones, span),
        },
        span,
    )
}

fn events_to_value(events: Vec<IcalEvent>, span: Span) -> Value {
    Value::list(
        events
            .into_iter()
            .map(|event| {
                Value::record(
                    record! {
                        "properties" => properties_to_value(event.properties, span),
                        "alarms" => alarms_to_value(event.alarms, span),
                    },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn alarms_to_value(alarms: Vec<IcalAlarm>, span: Span) -> Value {
    Value::list(
        alarms
            .into_iter()
            .map(|alarm| {
                Value::record(
                    record! { "properties" => properties_to_value(alarm.properties, span), },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn todos_to_value(todos: Vec<IcalTodo>, span: Span) -> Value {
    Value::list(
        todos
            .into_iter()
            .map(|todo| {
                Value::record(
                    record! {
                        "properties" => properties_to_value(todo.properties, span),
                        "alarms" => alarms_to_value(todo.alarms, span),
                    },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn journals_to_value(journals: Vec<IcalJournal>, span: Span) -> Value {
    Value::list(
        journals
            .into_iter()
            .map(|journal| {
                Value::record(
                    record! { "properties" => properties_to_value(journal.properties, span), },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn free_busys_to_value(free_busys: Vec<IcalFreeBusy>, span: Span) -> Value {
    Value::list(
        free_busys
            .into_iter()
            .map(|free_busy| {
                Value::record(
                    record! { "properties" => properties_to_value(free_busy.properties, span) },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn timezones_to_value(timezones: Vec<IcalTimeZone>, span: Span) -> Value {
    Value::list(
        timezones
            .into_iter()
            .map(|timezone| {
                Value::record(
                    record! {
                        "properties" => properties_to_value(timezone.properties, span),
                        "transitions" => timezone_transitions_to_value(timezone.transitions, span),
                    },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn timezone_transitions_to_value(transitions: Vec<IcalTimeZoneTransition>, span: Span) -> Value {
    Value::list(
        transitions
            .into_iter()
            .map(|transition| {
                Value::record(
                    record! { "properties" => properties_to_value(transition.properties, span) },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn properties_to_value(properties: Vec<Property>, span: Span) -> Value {
    Value::list(
        properties
            .into_iter()
            .map(|prop| {
                let name = Value::string(prop.name, span);
                let value = match prop.value {
                    Some(val) => Value::string(val, span),
                    None => Value::nothing(span),
                };
                let params = match prop.params {
                    Some(param_list) => params_to_value(param_list, span),
                    None => Value::nothing(span),
                };

                Value::record(
                    record! {
                        "name" => name,
                        "value" => value,
                        "params" => params,
                    },
                    span,
                )
            })
            .collect::<Vec<Value>>(),
        span,
    )
}

fn params_to_value(params: Vec<(String, Vec<String>)>, span: Span) -> Value {
    let mut row = IndexMap::new();

    for (param_name, param_values) in params {
        let values: Vec<Value> = param_values
            .into_iter()
            .map(|val| Value::string(val, span))
            .collect();
        let values = Value::list(values, span);
        row.insert(param_name, values);
    }

    Value::record(row.into_iter().collect(), span)
}
