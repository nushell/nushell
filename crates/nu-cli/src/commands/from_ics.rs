extern crate ical;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use ical::parser::ical::component::*;
use ical::property::Property;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use std::io::BufReader;

pub struct FromIcs;

impl WholeStreamCommand for FromIcs {
    fn name(&self) -> &str {
        "from-ics"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-ics")
    }

    fn usage(&self) -> &str {
        "Parse text as .ics and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_ics(args, registry)
    }
}

fn from_ics(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let input_string = input.collect_string(tag.clone()).await?.item;
        let input_bytes = input_string.as_bytes();
        let buf_reader = BufReader::new(input_bytes);
        let parser = ical::IcalParser::new(buf_reader);

        for calendar in parser {
            match calendar {
                Ok(c) => yield ReturnSuccess::value(calendar_to_value(c, tag.clone())),
                Err(_) => yield Err(ShellError::labeled_error(
                    "Could not parse as .ics",
                    "input cannot be parsed as .ics",
                    tag.clone()
                )),
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn calendar_to_value(calendar: IcalCalendar, tag: Tag) -> Value {
    let mut row = TaggedDictBuilder::new(tag.clone());

    row.insert_untagged(
        "properties",
        properties_to_value(calendar.properties, tag.clone()),
    );
    row.insert_untagged("events", events_to_value(calendar.events, tag.clone()));
    row.insert_untagged("alarms", alarms_to_value(calendar.alarms, tag.clone()));
    row.insert_untagged("to-Dos", todos_to_value(calendar.todos, tag.clone()));
    row.insert_untagged(
        "journals",
        journals_to_value(calendar.journals, tag.clone()),
    );
    row.insert_untagged(
        "free-busys",
        free_busys_to_value(calendar.free_busys, tag.clone()),
    );
    row.insert_untagged("timezones", timezones_to_value(calendar.timezones, tag));

    row.into_value()
}

fn events_to_value(events: Vec<IcalEvent>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &events
            .into_iter()
            .map(|event| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(event.properties, tag.clone()),
                );
                row.insert_untagged("alarms", alarms_to_value(event.alarms, tag.clone()));
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn alarms_to_value(alarms: Vec<IcalAlarm>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &alarms
            .into_iter()
            .map(|alarm| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(alarm.properties, tag.clone()),
                );
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn todos_to_value(todos: Vec<IcalTodo>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &todos
            .into_iter()
            .map(|todo| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(todo.properties, tag.clone()),
                );
                row.insert_untagged("alarms", alarms_to_value(todo.alarms, tag.clone()));
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn journals_to_value(journals: Vec<IcalJournal>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &journals
            .into_iter()
            .map(|journal| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(journal.properties, tag.clone()),
                );
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn free_busys_to_value(free_busys: Vec<IcalFreeBusy>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &free_busys
            .into_iter()
            .map(|free_busy| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(free_busy.properties, tag.clone()),
                );
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn timezones_to_value(timezones: Vec<IcalTimeZone>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &timezones
            .into_iter()
            .map(|timezone| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(timezone.properties, tag.clone()),
                );
                row.insert_untagged(
                    "transitions",
                    timezone_transitions_to_value(timezone.transitions, tag.clone()),
                );
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn timezone_transitions_to_value(
    transitions: Vec<IcalTimeZoneTransition>,
    tag: Tag,
) -> UntaggedValue {
    UntaggedValue::table(
        &transitions
            .into_iter()
            .map(|transition| {
                let mut row = TaggedDictBuilder::new(tag.clone());
                row.insert_untagged(
                    "properties",
                    properties_to_value(transition.properties, tag.clone()),
                );
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn properties_to_value(properties: Vec<Property>, tag: Tag) -> UntaggedValue {
    UntaggedValue::table(
        &properties
            .into_iter()
            .map(|prop| {
                let mut row = TaggedDictBuilder::new(tag.clone());

                let name = UntaggedValue::string(prop.name);
                let value = match prop.value {
                    Some(val) => UntaggedValue::string(val),
                    None => UntaggedValue::Primitive(Primitive::Nothing),
                };
                let params = match prop.params {
                    Some(param_list) => params_to_value(param_list, tag.clone()).into(),
                    None => UntaggedValue::Primitive(Primitive::Nothing),
                };

                row.insert_untagged("name", name);
                row.insert_untagged("value", value);
                row.insert_untagged("params", params);
                row.into_value()
            })
            .collect::<Vec<Value>>(),
    )
}

fn params_to_value(params: Vec<(String, Vec<String>)>, tag: Tag) -> Value {
    let mut row = TaggedDictBuilder::new(tag);

    for (param_name, param_values) in params {
        let values: Vec<Value> = param_values.into_iter().map(|val| val.into()).collect();
        let values = UntaggedValue::table(&values);
        row.insert_untagged(param_name, values);
    }

    row.into_value()
}
