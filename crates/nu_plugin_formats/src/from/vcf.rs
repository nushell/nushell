use ical::parser::vcard::component::*;
use ical::property::Property;
use indexmap::map::IndexMap;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, ShellError, Span, Spanned, SpannedValue};

pub const CMD_NAME: &str = "from vcf";

pub fn from_vcf_call(
    call: &EvaluatedCall,
    input: &SpannedValue,
) -> Result<SpannedValue, LabeledError> {
    let span = input.span().unwrap_or(call.head);
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
        Err(e) => SpannedValue::Error {
            error: Box::new(ShellError::UnsupportedInput(
                format!("input cannot be parsed as .vcf ({e})"),
                "value originates from here".into(),
                head,
                span,
            )),
        },
    });

    let collected: Vec<_> = iter.collect();
    Ok(SpannedValue::List {
        vals: collected,
        span: head,
    })
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
        result: Some(SpannedValue::List {
            vals: vec![SpannedValue::Record {
                cols: vec!["properties".to_string()],
                vals: vec![SpannedValue::List {
                    vals: vec![
                        SpannedValue::Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                SpannedValue::test_string("N"),
                                SpannedValue::test_string("Foo"),
                                SpannedValue::Nothing {
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                        SpannedValue::Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                SpannedValue::test_string("FN"),
                                SpannedValue::test_string("Bar"),
                                SpannedValue::Nothing {
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                        SpannedValue::Record {
                            cols: vec![
                                "name".to_string(),
                                "value".to_string(),
                                "params".to_string(),
                            ],
                            vals: vec![
                                SpannedValue::test_string("EMAIL"),
                                SpannedValue::test_string("foo@bar.com"),
                                SpannedValue::Nothing {
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        }),
    }]
}

fn contact_to_value(contact: VcardContact, span: Span) -> SpannedValue {
    let mut row = IndexMap::new();
    row.insert(
        "properties".to_string(),
        properties_to_value(contact.properties, span),
    );
    SpannedValue::from(Spanned { item: row, span })
}

fn properties_to_value(properties: Vec<Property>, span: Span) -> SpannedValue {
    SpannedValue::List {
        vals: properties
            .into_iter()
            .map(|prop| {
                let mut row = IndexMap::new();

                let name = SpannedValue::String {
                    val: prop.name,
                    span,
                };
                let value = match prop.value {
                    Some(val) => SpannedValue::String { val, span },
                    None => SpannedValue::Nothing { span },
                };
                let params = match prop.params {
                    Some(param_list) => params_to_value(param_list, span),
                    None => SpannedValue::Nothing { span },
                };

                row.insert("name".to_string(), name);
                row.insert("value".to_string(), value);
                row.insert("params".to_string(), params);
                SpannedValue::from(Spanned { item: row, span })
            })
            .collect::<Vec<SpannedValue>>(),
        span,
    }
}

fn params_to_value(params: Vec<(String, Vec<String>)>, span: Span) -> SpannedValue {
    let mut row = IndexMap::new();

    for (param_name, param_values) in params {
        let values: Vec<SpannedValue> = param_values
            .into_iter()
            .map(|val| SpannedValue::string(val, span))
            .collect();
        let values = SpannedValue::List { vals: values, span };
        row.insert(param_name, values);
    }

    SpannedValue::from(Spanned { item: row, span })
}
