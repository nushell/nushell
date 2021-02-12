use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromXML;

#[async_trait]
impl WholeStreamCommand for FromXML {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_xml(args).await
    }
}

fn from_attributes_to_value(attributes: &[roxmltree::Attribute], tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    let mut collected = TaggedDictBuilder::new(tag);
    for a in attributes {
        collected.insert_untagged(String::from(a.name()), UntaggedValue::string(a.value()));
    }

    collected.into_value()
}

fn from_node_to_value(n: &roxmltree::Node, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, &tag));
        }

        let children_values: Vec<Value> = children_values
            .into_iter()
            .filter(|x| match x {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(f)),
                    ..
                } => {
                    !f.trim().is_empty() // non-whitespace characters?
                }
                _ => true,
            })
            .collect();

        let mut collected = TaggedDictBuilder::new(&tag);

        let attribute_value: Value = from_attributes_to_value(&n.attributes(), &tag);

        let mut row = TaggedDictBuilder::new(&tag);
        row.insert_untagged(
            String::from("children"),
            UntaggedValue::Table(children_values),
        );
        row.insert_untagged(String::from("attributes"), attribute_value);
        collected.insert_untagged(name, row.into_value());

        collected.into_value()
    } else if n.is_comment() {
        UntaggedValue::string("<comment>").into_value(tag)
    } else if n.is_pi() {
        UntaggedValue::string("<processing_instruction>").into_value(tag)
    } else if n.is_text() {
        match n.text() {
            Some(text) => UntaggedValue::string(text).into_value(tag),
            None => UntaggedValue::string("<error>").into_value(tag),
        }
    } else {
        UntaggedValue::string("<unknown>").into_value(tag)
    }
}

fn from_document_to_value(d: &roxmltree::Document, tag: impl Into<Tag>) -> Value {
    from_node_to_value(&d.root_element(), tag)
}

pub fn from_xml_string_to_value(s: String, tag: impl Into<Tag>) -> Result<Value, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, tag))
}

async fn from_xml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;

    let concat_string = input.collect_string(tag.clone()).await?;

    Ok(
        match from_xml_string_to_value(concat_string.item, tag.clone()) {
            Ok(x) => match x {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => futures::stream::iter(list.into_iter().map(ReturnSuccess::value))
                    .to_output_stream(),
                x => OutputStream::one(ReturnSuccess::value(x)),
            },
            Err(_) => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as XML",
                    "input cannot be parsed as XML",
                    &tag,
                    "value originates from here",
                    &concat_string.tag,
                ))
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use crate::commands::from_xml;
    use indexmap::IndexMap;
    use nu_protocol::{UntaggedValue, Value};
    use nu_source::*;

    fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    fn row(entries: IndexMap<String, Value>) -> Value {
        UntaggedValue::row(entries).into_untagged_value()
    }

    fn table(list: &[Value]) -> Value {
        UntaggedValue::table(list).into_untagged_value()
    }

    fn parse(xml: &str) -> Result<Value, roxmltree::Error> {
        from_xml::from_xml_string_to_value(xml.to_string(), Tag::unknown())
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
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use super::FromXML;
        use crate::examples::test as test_examples;

        test_examples(FromXML {})
    }
}
