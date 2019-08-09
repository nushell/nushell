use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

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
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, tag))
}

pub fn from_xml(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let out = args.input;
    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_xml_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as XML",
                            "input cannot be parsed as XML",
                            span,
                            "value originates from here",
                            value_tag.span,
                        )),
                    }
                }
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    a.span(),
                )),
            }
        })
        .to_output_stream())
}
