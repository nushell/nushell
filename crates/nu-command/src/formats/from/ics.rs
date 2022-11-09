extern crate ical;
use ical::parser::ical::component::*;
use ical::property::Property;
use indexmap::map::IndexMap;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, Type, Value,
};
use std::io::BufReader;

#[derive(Clone)]
pub struct FromIcs;

impl Command for FromIcs {
    fn name(&self) -> &str {
        "from ics"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ics")
            .input_output_types(vec![(Type::String, Type::Table(vec![]))])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .ics and create table."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = engine_state.get_config();
        from_ics(input, head, config)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "'BEGIN:VCALENDAR
END:VCALENDAR' | from ics",
            description: "Converts ics formatted string to table",
            result: Some(Value::List {
                vals: vec![Value::Record {
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
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }],
                span: Span::test_data(),
            }),
        }]
    }
}

fn from_ics(input: PipelineData, head: Span, config: &Config) -> Result<PipelineData, ShellError> {
    let input_string = input.collect_string("", config)?;

    let input_string = input_string
        .lines()
        .map(|x| x.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let input_bytes = input_string.as_bytes();
    let buf_reader = BufReader::new(input_bytes);
    let parser = ical::IcalParser::new(buf_reader);

    let mut output = vec![];

    for calendar in parser {
        match calendar {
            Ok(c) => output.push(calendar_to_value(c, head)),
            Err(e) => output.push(Value::Error {
                error: ShellError::UnsupportedInput(
                    format!("input cannot be parsed as .ics ({})", e),
                    head,
                ),
            }),
        }
    }
    Ok(Value::List {
        vals: output,
        span: head,
    }
    .into_pipeline_data())
}

fn calendar_to_value(calendar: IcalCalendar, span: Span) -> Value {
    let mut row = IndexMap::new();

    row.insert(
        "properties".to_string(),
        properties_to_value(calendar.properties, span),
    );
    row.insert("events".to_string(), events_to_value(calendar.events, span));
    row.insert("alarms".to_string(), alarms_to_value(calendar.alarms, span));
    row.insert("to-Dos".to_string(), todos_to_value(calendar.todos, span));
    row.insert(
        "journals".to_string(),
        journals_to_value(calendar.journals, span),
    );
    row.insert(
        "free-busys".to_string(),
        free_busys_to_value(calendar.free_busys, span),
    );
    row.insert(
        "timezones".to_string(),
        timezones_to_value(calendar.timezones, span),
    );

    Value::from(Spanned { item: row, span })
}

fn events_to_value(events: Vec<IcalEvent>, span: Span) -> Value {
    Value::List {
        vals: events
            .into_iter()
            .map(|event| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(event.properties, span),
                );
                row.insert("alarms".to_string(), alarms_to_value(event.alarms, span));
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn alarms_to_value(alarms: Vec<IcalAlarm>, span: Span) -> Value {
    Value::List {
        vals: alarms
            .into_iter()
            .map(|alarm| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(alarm.properties, span),
                );
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn todos_to_value(todos: Vec<IcalTodo>, span: Span) -> Value {
    Value::List {
        vals: todos
            .into_iter()
            .map(|todo| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(todo.properties, span),
                );
                row.insert("alarms".to_string(), alarms_to_value(todo.alarms, span));
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn journals_to_value(journals: Vec<IcalJournal>, span: Span) -> Value {
    Value::List {
        vals: journals
            .into_iter()
            .map(|journal| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(journal.properties, span),
                );
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn free_busys_to_value(free_busys: Vec<IcalFreeBusy>, span: Span) -> Value {
    Value::List {
        vals: free_busys
            .into_iter()
            .map(|free_busy| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(free_busy.properties, span),
                );
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn timezones_to_value(timezones: Vec<IcalTimeZone>, span: Span) -> Value {
    Value::List {
        vals: timezones
            .into_iter()
            .map(|timezone| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(timezone.properties, span),
                );
                row.insert(
                    "transitions".to_string(),
                    timezone_transitions_to_value(timezone.transitions, span),
                );
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn timezone_transitions_to_value(transitions: Vec<IcalTimeZoneTransition>, span: Span) -> Value {
    Value::List {
        vals: transitions
            .into_iter()
            .map(|transition| {
                let mut row = IndexMap::new();
                row.insert(
                    "properties".to_string(),
                    properties_to_value(transition.properties, span),
                );
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn properties_to_value(properties: Vec<Property>, span: Span) -> Value {
    Value::List {
        vals: properties
            .into_iter()
            .map(|prop| {
                let mut row = IndexMap::new();

                let name = Value::String {
                    val: prop.name,
                    span,
                };
                let value = match prop.value {
                    Some(val) => Value::String { val, span },
                    None => Value::nothing(span),
                };
                let params = match prop.params {
                    Some(param_list) => params_to_value(param_list, span),
                    None => Value::nothing(span),
                };

                row.insert("name".to_string(), name);
                row.insert("value".to_string(), value);
                row.insert("params".to_string(), params);
                Value::from(Spanned { item: row, span })
            })
            .collect::<Vec<Value>>(),
        span,
    }
}

fn params_to_value(params: Vec<(String, Vec<String>)>, span: Span) -> Value {
    let mut row = IndexMap::new();

    for (param_name, param_values) in params {
        let values: Vec<Value> = param_values
            .into_iter()
            .map(|val| Value::string(val, span))
            .collect();
        let values = Value::List { vals: values, span };
        row.insert(param_name, values);
    }

    Value::from(Spanned { item: row, span })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromIcs {})
    }
}
