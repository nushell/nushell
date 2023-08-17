use eml_parser::eml::*;
use eml_parser::EmlParser;
use indexmap::map::IndexMap;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, ShellError, Span, Spanned, SpannedValue};

const DEFAULT_BODY_PREVIEW: usize = 50;
pub const CMD_NAME: &str = "from eml";

pub fn from_eml_call(
    call: &EvaluatedCall,
    input: &SpannedValue,
) -> Result<SpannedValue, LabeledError> {
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
            result: Some(SpannedValue::Record {
                cols: vec![
                    "Subject".to_string(),
                    "From".to_string(),
                    "To".to_string(),
                    "Body".to_string(),
                ],
                vals: vec![
                    SpannedValue::test_string("Welcome"),
                    SpannedValue::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            SpannedValue::nothing(Span::test_data()),
                            SpannedValue::test_string("test@email.com"),
                        ],
                        span: Span::test_data(),
                    },
                    SpannedValue::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            SpannedValue::nothing(Span::test_data()),
                            SpannedValue::test_string("someone@somewhere.com"),
                        ],
                        span: Span::test_data(),
                    },
                    SpannedValue::test_string("Test"),
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
            result: Some(SpannedValue::Record {
                cols: vec![
                    "Subject".to_string(),
                    "From".to_string(),
                    "To".to_string(),
                    "Body".to_string(),
                ],
                vals: vec![
                    SpannedValue::test_string("Welcome"),
                    SpannedValue::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            SpannedValue::nothing(Span::test_data()),
                            SpannedValue::test_string("test@email.com"),
                        ],
                        span: Span::test_data(),
                    },
                    SpannedValue::Record {
                        cols: vec!["Name".to_string(), "Address".to_string()],
                        vals: vec![
                            SpannedValue::nothing(Span::test_data()),
                            SpannedValue::test_string("someone@somewhere.com"),
                        ],
                        span: Span::test_data(),
                    },
                    SpannedValue::test_string("T"),
                ],
                span: Span::test_data(),
            }),
        },
    ]
}

fn emailaddress_to_value(span: Span, email_address: &EmailAddress) -> SpannedValue {
    let (n, a) = match email_address {
        EmailAddress::AddressOnly { address } => (
            SpannedValue::nothing(span),
            SpannedValue::String {
                val: address.to_string(),
                span,
            },
        ),
        EmailAddress::NameAndEmailAddress { name, address } => (
            SpannedValue::String {
                val: name.to_string(),
                span,
            },
            SpannedValue::String {
                val: address.to_string(),
                span,
            },
        ),
    };

    SpannedValue::Record {
        cols: vec!["Name".to_string(), "Address".to_string()],
        vals: vec![n, a],
        span,
    }
}

fn headerfieldvalue_to_value(head: Span, value: &HeaderFieldValue) -> SpannedValue {
    use HeaderFieldValue::*;

    match value {
        SingleEmailAddress(address) => emailaddress_to_value(head, address),
        MultipleEmailAddresses(addresses) => SpannedValue::List {
            vals: addresses
                .iter()
                .map(|a| emailaddress_to_value(head, a))
                .collect(),
            span: head,
        },
        Unstructured(s) => SpannedValue::string(s, head),
        Empty => SpannedValue::nothing(head),
    }
}

fn from_eml(
    input: &SpannedValue,
    body_preview: usize,
    head: Span,
) -> Result<SpannedValue, LabeledError> {
    let value = input.as_string()?;

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
        collected.insert(
            "Subject".to_string(),
            SpannedValue::String {
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
            SpannedValue::String {
                val: body,
                span: head,
            },
        );
    }

    Ok(SpannedValue::from(Spanned {
        item: collected,
        span: head,
    }))
}
