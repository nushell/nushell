use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::map::IndexMap;
use itertools::Itertools;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, Type,
    Value,
};
use roxmltree::NodeType;

#[derive(Clone)]
pub struct FromXml;

impl Command for FromXml {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .switch("keep-comments", "add comment nodes to result", None)
            .switch(
                "keep-processing-instructions",
                "add processing instruction nodes to result",
                None,
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create record."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let keep_comments = call.has_flag("keep-comments");
        let keep_processing_instructions = call.has_flag("keep-processing-instructions");
        let info = ParsingInfo {
            span: head,
            keep_comments,
            keep_processing_instructions
        };
        from_xml(input, &info)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: r#"'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml"#,
            description: "Converts xml formatted string to record",
            result: Some(Value::Record {
                cols: vec!["note".to_string()],
                vals: vec![Value::Record {
                    cols: vec!["children".to_string(), "attributes".to_string()],
                    vals: vec![
                        Value::List {
                            vals: vec![Value::Record {
                                cols: vec!["remember".to_string()],
                                vals: vec![Value::Record {
                                    cols: vec!["children".to_string(), "attributes".to_string()],
                                    vals: vec![
                                        Value::List {
                                            vals: vec![Value::test_string("Event")],
                                            span: Span::test_data(),
                                        },
                                        Value::Record {
                                            cols: vec![],
                                            vals: vec![],
                                            span: Span::test_data(),
                                        },
                                    ],
                                    span: Span::test_data(),
                                }],
                                span: Span::test_data(),
                            }],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec![],
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

struct ParsingInfo {
    span: Span,
    keep_comments: bool,
    keep_processing_instructions: bool,
}

fn from_attributes_to_value(attributes: &[roxmltree::Attribute], info: &ParsingInfo) -> Value {
    let mut collected = IndexMap::new();
    for a in attributes {
        collected.insert(String::from(a.name()), Value::string(a.value(), info.span));
    }

    let (cols, vals) = collected
        .into_iter()
        .fold((vec![], vec![]), |mut acc, (k, v)| {
            acc.0.push(k);
            acc.1.push(v);
            acc
        });

    Value::Record {
        cols,
        vals,
        span: info.span,
    }
}

fn element_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Value {
    let span = info.span;
    let mut node = IndexMap::new();

    let tag = n.tag_name().name().trim().to_string();
    let tag = Value::string(tag, span);

    let content: Vec<Value> = n
        .children()
        .into_iter()
        .filter_map(|node| from_node_to_value(&node, info))
        .collect();
    let content = Value::list(content, span);

    let attributes = from_attributes_to_value(&n.attributes().collect::<Vec<_>>(), info);

    node.insert(String::from(COLUMN_TAG_NAME), tag);
    node.insert(String::from(COLUMN_ATTRS_NAME), attributes);
    node.insert(String::from(COLUMN_CONTENT_NAME), content);

    Value::from(Spanned { item: node, span })
}

fn text_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    let span = info.span;
    let text = n.text().expect("Non-text node supplied to text_to_value");
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        let mut node = IndexMap::new();
        let content = Value::string(String::from(text), span);

        node.insert(String::from(COLUMN_TAG_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = Value::from(Spanned { item: node, span });

        Some(result)
    }
}

fn comment_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    if info.keep_comments {
        let span = info.span;
        let text = n.text().expect("Non-comment node supplied to comment_to_value");

        let mut node = IndexMap::new();
        let content = Value::string(String::from(text), span);

        node.insert(String::from(COLUMN_TAG_NAME), Value::string("!", span));
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = Value::from(Spanned { item: node, span });

        Some(result)
    } else {
        None
    }
}

fn processing_instruction_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    if info.keep_processing_instructions {
        let span = info.span;
        let pi = n.pi()?;

        let mut node = IndexMap::new();
        // Add '?' before target to differentiate tags from pi targets
        let tag = format!("?{}", pi.target);
        let tag = Value::string(tag, span);
        let content = pi.value.map_or_else(|| {Value::nothing(span)}, |x| {Value::string(x, span)});

        node.insert(String::from(COLUMN_TAG_NAME), tag);
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = Value::from(Spanned { item: node, span });

        Some(result)
    } else {
        None
    }
}

fn from_node_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    match n.node_type() {
        NodeType::Element => Some(element_to_value(n, info)),
        NodeType::Text => text_to_value(n, info),
        NodeType::Comment => comment_to_value(n, info),
        NodeType::PI => processing_instruction_to_value(n, info),
        _ => None,
    }
}

fn from_document_to_value(d: &roxmltree::Document, info: &ParsingInfo) -> Value {
    element_to_value(&d.root_element(), info)
}

fn from_xml_string_to_value(s: String, info: &ParsingInfo) -> Result<Value, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, info))
}

fn from_xml(input: PipelineData, info: &ParsingInfo) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(info.span)?;

    match from_xml_string_to_value(concat_string, info) {
        Ok(x) => Ok(x.into_pipeline_data_with_metadata(metadata)),
        _ => Err(ShellError::UnsupportedInput(
            "Could not parse string as XML".to_string(),
            "value originates from here".into(),
            info.span,
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indexmap::indexmap;
    use indexmap::IndexMap;
    use nu_protocol::{Spanned, Value};

    fn string(input: impl Into<String>) -> Value {
        Value::test_string(input)
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        Value::from(Spanned {
            item: entries,
            span: Span::test_data(),
        })
    }

    fn table(list: &[Value]) -> Value {
        Value::List {
            vals: list.to_vec(),
            span: Span::test_data(),
        }
    }

    fn parse(xml: &str) -> Result<Value, roxmltree::Error> {
        let info = ParsingInfo {
            span: Span::test_data(),
            keep_comments: false,
            keep_processing_instructions: false,
        };
        from_xml_string_to_value(xml.to_string(), &info)
    }

    #[test]
    fn parses_empty_element() -> Result<(), roxmltree::Error> {
        let source = "<nu></nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[]),
                    "attributes".into() => row(indexmap! {})
                })
            })
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_text() -> Result<(), roxmltree::Error> {
        let source = "<nu>La era de los tres caballeros</nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[string("La era de los tres caballeros")]),
                    "attributes".into() => row(indexmap! {})
                })
            })
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_elements() -> Result<(), roxmltree::Error> {
        let source = "\
<nu>
    <dev>Andrés</dev>
    <dev>Jonathan</dev>
    <dev>Yehuda</dev>
</nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[
                        row(indexmap! {
                            "dev".into() => row(indexmap! {
                                "children".into() => table(&[string("Andrés")]),
                                "attributes".into() => row(indexmap! {})
                            })
                        }),
                        row(indexmap! {
                            "dev".into() => row(indexmap! {
                                "children".into() => table(&[string("Jonathan")]),
                                "attributes".into() => row(indexmap! {})
                            })
                        }),
                        row(indexmap! {
                            "dev".into() => row(indexmap! {
                                "children".into() => table(&[string("Yehuda")]),
                                "attributes".into() => row(indexmap! {})
                            })
                        })
                    ]),
                    "attributes".into() => row(indexmap! {})
                })
            })
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_attribute() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\">
</nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[]),
                    "attributes".into() => row(indexmap! {
                        "version".into() => string("2.0")
                    })
                })
            })
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_attribute_and_element() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\">
    <version>2.0</version>
</nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[
                           row(indexmap! {
                                "version".into() => row(indexmap! {
                                    "children".into() => table(&[string("2.0")]),
                                    "attributes".into() => row(indexmap! {})
                                })
                          })
                    ]),
                    "attributes".into() => row(indexmap! {
                        "version".into() => string("2.0")
                    })
                })
            })
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_multiple_attributes() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\" age=\"25\">
</nu>";

        assert_eq!(
            parse(source)?,
            row(indexmap! {
                "nu".into() => row(indexmap! {
                    "children".into() => table(&[]),
                    "attributes".into() => row(indexmap! {
                        "version".into() => string("2.0"),
                        "age".into() => string("25")
                    })
                })
            })
        );

        Ok(())
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromXml {})
    }
}
