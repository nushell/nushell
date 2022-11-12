use indexmap::map::IndexMap;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, Type, Value,
};

#[derive(Clone)]
pub struct FromXml;

impl Command for FromXml {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create record."
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
        from_xml(input, head, config)
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
                                            vals: vec![Value::String {
                                                val: "Event".to_string(),
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

fn from_attributes_to_value(attributes: &[roxmltree::Attribute], span: Span) -> Value {
    let mut collected = IndexMap::new();
    for a in attributes {
        collected.insert(String::from(a.name()), Value::string(a.value(), span));
    }

    let (cols, vals) = collected
        .into_iter()
        .fold((vec![], vec![]), |mut acc, (k, v)| {
            acc.0.push(k);
            acc.1.push(v);
            acc
        });

    Value::Record { cols, vals, span }
}

fn from_node_to_value(n: &roxmltree::Node, span: Span) -> Value {
    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, span));
        }

        let children_values: Vec<Value> = children_values
            .into_iter()
            .filter(|x| match x {
                Value::String { val: f, .. } => {
                    !f.trim().is_empty() // non-whitespace characters?
                }
                _ => true,
            })
            .collect();

        let mut collected = IndexMap::new();

        let attribute_value: Value = from_attributes_to_value(n.attributes(), span);

        let mut row = IndexMap::new();
        row.insert(
            String::from("children"),
            Value::List {
                vals: children_values,
                span,
            },
        );
        row.insert(String::from("attributes"), attribute_value);
        collected.insert(name, Value::from(Spanned { item: row, span }));

        Value::from(Spanned {
            item: collected,
            span,
        })
    } else if n.is_comment() {
        Value::String {
            val: "<comment>".to_string(),
            span,
        }
    } else if n.is_pi() {
        Value::String {
            val: "<processing_instruction>".to_string(),
            span,
        }
    } else if n.is_text() {
        match n.text() {
            Some(text) => Value::String {
                val: text.to_string(),
                span,
            },
            None => Value::String {
                val: "<error>".to_string(),
                span,
            },
        }
    } else {
        Value::String {
            val: "<unknown>".to_string(),
            span,
        }
    }
}

fn from_document_to_value(d: &roxmltree::Document, span: Span) -> Value {
    from_node_to_value(&d.root_element(), span)
}

pub fn from_xml_string_to_value(s: String, span: Span) -> Result<Value, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, span))
}

fn from_xml(input: PipelineData, head: Span, config: &Config) -> Result<PipelineData, ShellError> {
    let concat_string = input.collect_string("", config)?;

    match from_xml_string_to_value(concat_string, head) {
        Ok(x) => Ok(x.into_pipeline_data()),
        _ => Err(ShellError::UnsupportedInput(
            "Could not parse string as xml".to_string(),
            head,
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
        Value::String {
            val: input.into(),
            span: Span::test_data(),
        }
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
        from_xml_string_to_value(xml.to_string(), Span::test_data())
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
