use crate::FormatCmdsPlugin;
use eml_parser::EmlParser;
use eml_parser::eml::*;
use indexmap::IndexMap;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, ShellError, Signature, Span, SyntaxShape, Type, Value, record,
};

const DEFAULT_BODY_PREVIEW: usize = 50;

pub struct FromEml;

impl SimplePluginCommand for FromEml {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "from eml"
    }

    fn description(&self) -> &str {
        "Parse text as .eml and create record."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::record())])
            .named(
                "preview-body",
                SyntaxShape::Int,
                "How many bytes of the body to preview",
                Some('b'),
            )
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
        let preview_body: usize = call
            .get_flag::<i64>("preview-body")?
            .map(|l| if l < 0 { 0 } else { l as usize })
            .unwrap_or(DEFAULT_BODY_PREVIEW);
        from_eml(input, preview_body, call.head)
    }
}

pub fn examples() -> Vec<Example<'static>> {
    vec![
        Example {
            description: "Convert eml structured data into record",
            example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml",
            result: Some(Value::test_record(record! {
                    "Subject" => Value::test_string("Welcome"),
                    "From" =>    Value::test_record(record! {
                        "Name" =>        Value::nothing(Span::test_data()),
                        "Address" =>     Value::test_string("test@email.com"),
                    }),
                    "To" => Value::test_record(record! {
                        "Name" =>        Value::nothing(Span::test_data()),
                        "Address" =>     Value::test_string("someone@somewhere.com"),
                    }),
                    "Body" => Value::test_string("Test"),
            })),
        },
        Example {
            description: "Convert eml structured data into record",
            example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml -b 1",
            result: Some(Value::test_record(record! {
                    "Subject" => Value::test_string("Welcome"),
                    "From" =>    Value::test_record(record! {
                        "Name" =>          Value::nothing(Span::test_data()),
                        "Address" =>       Value::test_string("test@email.com"),
                    }),
                    "To" => Value::test_record(record! {
                        "Name" =>        Value::nothing(Span::test_data()),
                        "Address" =>     Value::test_string("someone@somewhere.com"),
                    }),
                    "Body" => Value::test_string("T"),
            })),
        },
    ]
}

fn emailaddress_to_value(span: Span, email_address: &EmailAddress) -> Value {
    let (n, a) = match email_address {
        EmailAddress::AddressOnly { address } => {
            (Value::nothing(span), Value::string(address, span))
        }
        EmailAddress::NameAndEmailAddress { name, address } => {
            (Value::string(name, span), Value::string(address, span))
        }
    };

    Value::record(
        record! {
            "Name" => n,
            "Address" => a,
        },
        span,
    )
}

fn headerfieldvalue_to_value(head: Span, value: &HeaderFieldValue) -> Value {
    use HeaderFieldValue::*;

    match value {
        SingleEmailAddress(address) => emailaddress_to_value(head, address),
        MultipleEmailAddresses(addresses) => Value::list(
            addresses
                .iter()
                .map(|a| emailaddress_to_value(head, a))
                .collect(),
            head,
        ),
        Unstructured(s) => Value::string(s, head),
        Empty => Value::nothing(head),
    }
}

fn from_eml(input: &Value, body_preview: usize, head: Span) -> Result<Value, LabeledError> {
    let value = input.coerce_string()?;

    let eml = EmlParser::from_string(value)
        .with_body_preview(body_preview)
        .parse()
        .map_err(|_| ShellError::CantConvert {
            to_type: "structured eml data".into(),
            from_type: "string".into(),
            span: head,
            help: None,
        })?;

    let mut collected = IndexMap::new();

    if let Some(subj) = eml.subject {
        collected.insert("Subject".to_string(), Value::string(subj, head));
    }

    if let Some(from) = eml.from {
        collected.insert("From".to_string(), headerfieldvalue_to_value(head, &from));
    }

    if let Some(to) = eml.to {
        collected.insert("To".to_string(), headerfieldvalue_to_value(head, &to));
    }

    for HeaderField { name, value } in &eml.headers {
        collected.insert(name.to_string(), headerfieldvalue_to_value(head, value));
    }

    if let Some(body) = eml.body {
        collected.insert("Body".to_string(), Value::string(body, head));
    }

    Ok(Value::record(collected.into_iter().collect(), head))
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("formats", crate::FormatCmdsPlugin.into())?.test_command_examples(&FromEml)
}
