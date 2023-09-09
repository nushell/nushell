use ical::parser::vcard::component::*;
use ical::property::Property;
use indexmap::map::IndexMap;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{record, PluginExample, Record, ShellError, Span, Value};

pub const CMD_NAME: &str = "from vcf";

pub fn from_vcf_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let span = input.span();
    let input_string = input.as_string()?;
    let head = call.head;

    let input_string = input_string
        .lines()
        .map(|x| x.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let input_bytes = input_string.as_bytes();
    let cursor = std::io::Cursor::new(input_bytes);
    let parser = ical::VcardParser::new(cursor);

    let iter = parser.map(move |contact| match contact {
        Ok(c) => contact_to_value(c, head),
        Err(e) => Value::error(
            ShellError::UnsupportedInput(
                format!("input cannot be parsed as .vcf ({e})"),
                "value originates from here".into(),
                head,
                span,
            ),
            span,
        ),
    });

    let collected: Vec<_> = iter.collect();
    Ok(Value::list(collected, head))
}

pub fn examples() -> Vec<PluginExample> {
    vec![PluginExample {
        example: "'BEGIN:VCARD
N:Foo
FN:Bar
EMAIL:foo@bar.com
END:VCARD' | from vcf"
            .into(),
        description: "Converts ics formatted string to table".into(),
        result: Some(Value::list(
            vec![Value::test_record(Record {
                cols: vec!["properties".to_string()],
                vals: vec![Value::list(
                    vec![
                        Value::test_record(Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                Value::test_string("N"),
                                Value::test_string("Foo"),
                                Value::nothing(Span::test_data()),
                            ],
                        }),
                        Value::test_record(Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                Value::test_string("FN"),
                                Value::test_string("Bar"),
                                Value::nothing(Span::test_data()),
                            ],
                        }),
                        Value::test_record(Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                Value::test_string("EMAIL"),
                                Value::test_string("foo@bar.com"),
                                Value::nothing(Span::test_data()),
                            ],
                        }),
                    ],
                    Span::test_data(),
                )],
            })],
            Span::test_data(),
        )),
    }]
}

fn contact_to_value(contact: VcardContact, span: Span) -> Value {
    Value::record(
        record! { "properties" => properties_to_value(contact.properties, span) },
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
