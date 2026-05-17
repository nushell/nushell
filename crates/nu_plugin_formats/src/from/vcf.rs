use crate::FormatCmdsPlugin;

use ical::{parser::vcard::component::*, property::Property};
use indexmap::IndexMap;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, ShellError, Signature, Span, Type, Value, record,
};

pub struct FromVcf;

impl SimplePluginCommand for FromVcf {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "from vcf"
    }

    fn description(&self) -> &str {
        "Parse text as .vcf and create table."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::table())])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        examples()
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        let input_string = input.coerce_str()?;
        let head = call.head;

        let input_string = input_string
            .lines()
            .enumerate()
            .map(|(i, x)| {
                if i == 0 {
                    x.trim().to_string()
                } else if x.len() > 1 && (x.starts_with(' ') || x.starts_with('\t')) {
                    x[1..].trim_end().to_string()
                } else {
                    format!("\n{}", x.trim())
                }
            })
            .collect::<String>();

        let input_bytes = input_string.as_bytes();
        let cursor = std::io::Cursor::new(input_bytes);
        let parser = ical::VcardParser::new(cursor);

        let iter = parser.map(move |contact| match contact {
            Ok(c) => contact_to_value(c, head),
            Err(e) => Value::error(
                ShellError::UnsupportedInput {
                    msg: format!("input cannot be parsed as .vcf ({e})"),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                },
                span,
            ),
        });

        let collected: Vec<_> = iter.collect();
        Ok(Value::list(collected, head))
    }
}

pub fn examples() -> Vec<Example<'static>> {
    vec![Example {
        example: "'BEGIN:VCARD
N:Foo
FN:Bar
EMAIL:foo@bar.com
END:VCARD' | from vcf",
        description: "Converts ics formatted string to table",
        result: Some(Value::test_list(vec![Value::test_record(record! {
            "properties" => Value::test_list(
                vec![
                    Value::test_record(record! {
                            "name" =>   Value::test_string("N"),
                            "value" =>  Value::test_string("Foo"),
                            "params" => Value::nothing(Span::test_data()),
                    }),
                    Value::test_record(record! {
                            "name" =>   Value::test_string("FN"),
                            "value" =>  Value::test_string("Bar"),
                            "params" => Value::nothing(Span::test_data()),
                    }),
                    Value::test_record(record! {
                            "name" =>   Value::test_string("EMAIL"),
                            "value" =>  Value::test_string("foo@bar.com"),
                            "params" => Value::nothing(Span::test_data()),
                    }),
                ],
            ),
        })])),
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

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("formats", crate::FormatCmdsPlugin.into())?.test_command_examples(&FromVcf)
}
