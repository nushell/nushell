use crate::object::{Primitive, SpannedDictBuilder, Value};
use crate::prelude::*;

fn from_node_to_value<'a, 'd>(
    n: &roxmltree::Node<'a, 'd>,
    span: impl Into<Span>,
) -> Spanned<Value> {
    let span = span.into();

    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c, span));
        }

        let children_values: Vec<Spanned<Value>> = children_values
            .into_iter()
            .filter(|x| match x {
                Spanned {
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

        let mut collected = SpannedDictBuilder::new(span);
        collected.insert(name.clone(), Value::List(children_values));

        collected.into_spanned_value()
    } else if n.is_comment() {
        Value::string("<comment>").spanned(span)
    } else if n.is_pi() {
        Value::string("<processing_instruction>").spanned(span)
    } else if n.is_text() {
        Value::string(n.text().unwrap()).spanned(span)
    } else {
        Value::string("<unknown>").spanned(span)
    }
}

fn from_document_to_value(d: &roxmltree::Document, span: impl Into<Span>) -> Spanned<Value> {
    from_node_to_value(&d.root_element(), span)
}

pub fn from_xml_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Spanned<Value>, Box<dyn std::error::Error>> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, span))
}

pub fn from_xml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_xml_string_to_value(s, span) {
                Ok(x) => ReturnSuccess::value(x.spanned(a.span)),
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
