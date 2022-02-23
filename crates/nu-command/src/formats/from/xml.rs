use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct FromXml;

impl Command for FromXml {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
            .switch(
                "flatten",
                "use flatter/smaller representation where attributes are joined with nodes",
                Some('f'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create table."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = stack.get_config().unwrap_or_default();
        let flat = call.has_flag("flatten");
        from_xml(input, head, &config, flat)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: r#"'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml"#,
            description: "Converts xml formatted string to table",
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

fn add_row(cols: &mut Vec<String>, vals: &mut Vec<Value>, name: String, value: Value, span: Span) {
    for (idx, col) in cols.iter_mut().enumerate() {
        if col == &name {
            match &mut vals[idx] {
                Value::List { vals, .. } => {
                    vals.push(value);
                }
                x => {
                    let prev = x.clone();

                    *x = Value::List {
                        vals: vec![prev],
                        span,
                    }
                }
            }
            return;
        }
    }

    cols.push(name);
    vals.push(value);
}

// fn from_attributes_to_value(attributes: &[roxmltree::Attribute], span: Span) -> Value {
//     let mut collected = IndexMap::new();
//     for a in attributes {
//         collected.insert(String::from(a.name()), Value::string(a.value(), span));
//     }

//     let (cols, vals) = collected
//         .into_iter()
//         .fold((vec![], vec![]), |mut acc, (k, v)| {
//             acc.0.push(k);
//             acc.1.push(v);
//             acc
//         });

//     Value::Record { cols, vals, span }
// }

fn from_node_to_value(n: &roxmltree::Node, span: Span, flat: bool) -> (String, Value) {
    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, span, flat));
        }

        let children_values: Vec<(String, Value)> = children_values
            .into_iter()
            .filter(|x| match &x.1 {
                Value::String { val: f, .. } => {
                    !f.trim().is_empty() // non-whitespace characters?
                }
                _ => true,
            })
            .collect();

        let mut row_cols = vec![];
        let mut row_vals = vec![];

        if flat {
            for a in n.attributes() {
                add_row(
                    &mut row_cols,
                    &mut row_vals,
                    a.name().to_string(),
                    Value::String {
                        val: a.value().to_string(),
                        span,
                    },
                    span,
                );
            }

            for (children_col, children_val) in children_values {
                add_row(
                    &mut row_cols,
                    &mut row_vals,
                    children_col,
                    children_val,
                    span,
                );
            }
        } else {
            let attribute_value: Value = {
                let mut collected_cols = vec![];
                let mut collected_vals = vec![];
                for a in n.attributes() {
                    collected_cols.push(a.name().to_string());
                    collected_vals.push(Value::String {
                        val: a.value().to_string(),
                        span,
                    });
                }

                Value::Record {
                    cols: collected_cols,
                    vals: collected_vals,
                    span,
                }
            };
            row_cols.push("attributes".to_string());
            row_vals.push(attribute_value);

            let mut children_cols = vec![];
            let mut children_vals = vec![];

            for (children_col, children_val) in children_values {
                children_cols.push(children_col);
                children_vals.push(children_val);
            }

            row_cols.push("children".to_string());
            row_vals.push(Value::Record {
                cols: children_cols,
                vals: children_vals,
                span,
            });
        }

        (
            name,
            Value::Record {
                cols: row_cols,
                vals: row_vals,
                span,
            },
        )
    } else if n.is_comment() {
        (
            String::new(),
            Value::String {
                val: "<comment>".to_string(),
                span,
            },
        )
    } else if n.is_pi() {
        (
            String::new(),
            Value::String {
                val: "<processing_instruction>".to_string(),
                span,
            },
        )
    } else if n.is_text() {
        match n.text() {
            Some(text) => (
                text.to_string(),
                Value::String {
                    val: text.to_string(),
                    span,
                },
            ),
            None => (
                "<error>".to_string(),
                Value::String {
                    val: "<error>".to_string(),
                    span,
                },
            ),
        }
    } else {
        (
            "<unknown>".to_string(),
            Value::String {
                val: "<unknown>".to_string(),
                span,
            },
        )
    }
}

fn from_document_to_value(d: &roxmltree::Document, span: Span, flat: bool) -> Value {
    let (_, output) = from_node_to_value(&d.root_element(), span, flat);
    output
}

pub fn from_xml_string_to_value(
    s: String,
    span: Span,
    flat: bool,
) -> Result<Value, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, span, flat))
}

fn from_xml(
    input: PipelineData,
    head: Span,
    config: &Config,
    flat: bool,
) -> Result<PipelineData, ShellError> {
    let concat_string = input.collect_string("", config)?;

    match from_xml_string_to_value(concat_string, head, flat) {
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
        from_xml_string_to_value(xml.to_string(), Span::test_data(), false)
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
