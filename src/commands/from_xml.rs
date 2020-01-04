use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

pub struct FromXML;

impl WholeStreamCommand for FromXML {
    fn name(&self) -> &str {
        "from-xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-xml")
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_xml(args, registry)
    }
}

fn from_node_to_value<'a, 'd>(n: &roxmltree::Node<'a, 'd>, tag: impl Into<Tag>) -> Value {
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

        let mut collected = TaggedDictBuilder::new(tag);
        collected.insert_untagged(name, UntaggedValue::Table(children_values));

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

fn from_xml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let name_span = tag.span;
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            latest_tag = Some(value.tag.clone());
            let value_span = value.tag.span;

            if let Ok(s) = value.as_string() {
                concat_string.push_str(&s);
            }
            else {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    value_span,
                ))
            }
        }

        match from_xml_string_to_value(concat_string, tag.clone()) {
            Ok(x) => match x {
                Value { value: UntaggedValue::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as XML",
                    "input cannot be parsed as XML",
                    &tag,
                    "value originates from here",
                    &last_tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {

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
                "nu".into() => table(&[])
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
                "nu".into() => table(&[string("La era de los tres caballeros")])
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
                "nu".into() => table(&[
                    row(indexmap! {"dev".into() => table(&[string("Andrés")])}),
                    row(indexmap! {"dev".into() => table(&[string("Jonathan")])}),
                    row(indexmap! {"dev".into() => table(&[string("Yehuda")])})
                ])
            })
        );

        Ok(())
    }
}
