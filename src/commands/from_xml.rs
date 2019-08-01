use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

fn from_node_to_value<'a, 'd>(n: &roxmltree::Node<'a, 'd>, span: impl Into<Span>) -> Tagged<Value> {
    let span = span.into();

    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, span));
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

        let mut collected = TaggedDictBuilder::new(span);
        collected.insert(name.clone(), Value::List(children_values));

        collected.into_tagged_value()
    } else if n.is_comment() {
        Value::string("<comment>").tagged(span)
    } else if n.is_pi() {
        Value::string("<processing_instruction>").tagged(span)
    } else if n.is_text() {
        Value::string(n.text().unwrap()).tagged(span)
    } else {
        Value::string("<unknown>").tagged(span)
    }
}

fn from_document_to_value(d: &roxmltree::Document, span: impl Into<Span>) -> Tagged<Value> {
    from_node_to_value(&d.root_element(), span)
}

pub fn from_xml_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, span))
}

pub fn from_xml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;
    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_xml_string_to_value(s, span) {
                Ok(x) => ReturnSuccess::value(x),
                Err(_) => Err(ShellError::maybe_labeled_error(
                    "Could not parse as XML",
                    "piped data failed XML parse",
                    span,
                )),
            },
            _ => Err(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )),
        })
        .to_output_stream())
}
