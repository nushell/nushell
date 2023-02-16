use eml_parser::eml::*;
use eml_parser::EmlParser;
use indexmap::map::IndexMap;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, ShellError, Span, Spanned, Value};

const DEFAULT_BODY_PREVIEW: usize = 50;
pub const CMD_NAME: &str = "from eml";

pub fn from_eml_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let preview_body: usize = call
        .get_flag::<i64>("preview-body")?
        .map(|l| if l < 0 { 0 } else { l as usize })
        .unwrap_or(DEFAULT_BODY_PREVIEW);
    from_eml(input, preview_body, call.head)
}

pub fn examples() -> Vec<PluginExample> {
    vec![
        PluginExample {
            description: "Convert eml structured data into record".into(),
            example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml"
                .into(),
            result: Some(Value::Record {
                cols: vec![
                    "Subject".to_string(),
                    "From".to_string(),
                    "To".to_string(),
                    "Body".to_string(),
                ],
                vals: vec![
                    Value::test_string("Welcome"),
                    Value::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            Value::nothing(Span::test_data()),
                            Value::test_string("test@email.com"),
                        ],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            Value::nothing(Span::test_data()),
                            Value::test_string("someone@somewhere.com"),
                        ],
                        span: Span::test_data(),
                    },
                    Value::test_string("Test"),
                ],
                span: Span::test_data(),
            }),
        },
        PluginExample {
            description: "Convert eml structured data into record".into(),
            example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com
Test' | from eml -b 1"
                .into(),
            result: Some(Value::Record {
                cols: vec![
                    "Subject".to_string(),
                    "From".to_string(),
                    "To".to_string(),
                    "Body".to_string(),
                ],
                vals: vec![
                    Value::test_string("Welcome"),
                    Value::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            Value::nothing(Span::test_data()),
                            Value::test_string("test@email.com"),
                        ],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            Value::nothing(Span::test_data()),
                            Value::test_string("someone@somewhere.com"),
                        ],
                        span: Span::test_data(),
                    },
                    Value::test_string("T"),
                ],
                span: Span::test_data(),
            }),
        },
    ]
}

fn emailaddress_to_value(span: Span, email_address: &EmailAddress) -> Value {
    let (n, a) = match email_address {
        EmailAddress::AddressOnly { address } => (
            Value::nothing(span),
            Value::String {
                val: address.to_string(),
                span,
            },
        ),
        EmailAddress::NameAndEmailAddress { name, address } => (
            Value::String {
                val: name.to_string(),
                span,
            },
            Value::String {
                val: address.to_string(),
                span,
            },
        ),
    };

    Value::Record {
        cols: vec!["Name".to_string(), "Address".to_string()],
        vals: vec![n, a],
        span,
    }
}

fn headerfieldvalue_to_value(head: Span, value: &HeaderFieldValue) -> Value {
    use HeaderFieldValue::*;

    match value {
        SingleEmailAddress(address) => emailaddress_to_value(head, address),
        MultipleEmailAddresses(addresses) => Value::List {
            vals: addresses
                .iter()
                .map(|a| emailaddress_to_value(head, a))
                .collect(),
            span: head,
        },
        Unstructured(s) => Value::string(s, head),
        Empty => Value::nothing(head),
    }
}

fn from_eml(input: &Value, body_preview: usize, head: Span) -> Result<Value, LabeledError> {
    let value = input.as_string()?;

    let eml = EmlParser::from_string(value)
        .with_body_preview(body_preview)
        .parse()
        .map_err(|_| {
            ShellError::CantConvert("structured eml data".into(), "string".into(), head, None)
        })?;

    let mut collected = IndexMap::new();

    if let Some(subj) = eml.subject {
        collected.insert(
            "Subject".to_string(),
            Value::String {
                val: subj,
                span: head,
            },
        );
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
        collected.insert(
            "Body".to_string(),
            Value::String {
                val: body,
                span: head,
            },
        );
    }

    Ok(Value::from(Spanned {
        item: collected,
        span: head,
    }))
}
