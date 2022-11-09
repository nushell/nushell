use ::eml_parser::eml::*;
use ::eml_parser::EmlParser;
use indexmap::map::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::Config;
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct FromEml;

const DEFAULT_BODY_PREVIEW: usize = 50;

impl Command for FromEml {
    fn name(&self) -> &str {
        "from eml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from eml")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .named(
                "preview-body",
                SyntaxShape::Int,
                "How many bytes of the body to preview",
                Some('b'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .eml and create record."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let preview_body: Option<Spanned<i64>> =
            call.get_flag(engine_state, stack, "preview-body")?;
        let config = engine_state.get_config();
        from_eml(input, preview_body, head, config)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert eml structured data into record",
                example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml",
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
            Example {
                description: "Convert eml structured data into record",
                example: "'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml -b 1",
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
        Unstructured(s) => Value::String {
            val: s.to_string(),
            span: head,
        },
        Empty => Value::nothing(head),
    }
}

fn from_eml(
    input: PipelineData,
    preview_body: Option<Spanned<i64>>,
    head: Span,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let value = input.collect_string("", config)?;

    let body_preview = preview_body
        .map(|b| b.item as usize)
        .unwrap_or(DEFAULT_BODY_PREVIEW);

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

    Ok(PipelineData::Value(
        Value::from(Spanned {
            item: collected,
            span: head,
        }),
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromEml {})
    }
}
