use crate::commands::WholeStreamCommand;
use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

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

fn from_node_to_value<'a, 'd>(n: &roxmltree::Node<'a, 'd>, tag: impl Into<Tag>) -> Tagged<Value> {
    let tag = tag.into();

    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, tag));
        }

        let children_values: Vec<Tagged<Value>> = children_values
            .into_iter()
            .filter(|x| match x {
                Tagged {
                    item: Value::Primitive(Primitive::String(f)),
                    ..
                } => {
                    if f.trim() == "" {
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            })
            .collect();

        let mut collected = TaggedDictBuilder::new(tag);
        collected.insert(name.clone(), Value::List(children_values));

        collected.into_tagged_value()
    } else if n.is_comment() {
        Value::string("<comment>").tagged(tag)
    } else if n.is_pi() {
        Value::string("<processing_instruction>").tagged(tag)
    } else if n.is_text() {
        Value::string(n.text().unwrap()).tagged(tag)
    } else {
        Value::string("<unknown>").tagged(tag)
    }
}

fn from_document_to_value(d: &roxmltree::Document, tag: impl Into<Tag>) -> Tagged<Value> {
    from_node_to_value(&d.root_element(), tag)
}

pub fn from_xml_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, tag))
}

fn from_xml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let input = args.input;

    let stream = async_stream_block! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag);
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    value_tag.span,
                )),

            }
        }

        match from_xml_string_to_value(concat_string, span) {
            Ok(x) => match x {
                Tagged { item: Value::List(list), .. } => {
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
                    span,
                    "value originates from here",
                    last_tag.span,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
