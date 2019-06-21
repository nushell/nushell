use crate::object::{DataDescriptor, Dictionary, Primitive, Value};
use crate::prelude::*;

fn from_node_to_value<'a, 'd>(n: &roxmltree::Node<'a, 'd>) -> Value {
    if n.is_element() {
        let name = n.tag_name().name().trim().to_string();

        let mut children_values = vec![];
        for c in n.children() {
            children_values.push(from_node_to_value(&c));
        }

        let children_values: Vec<Value> = children_values
            .into_iter()
            .filter(|x| match x {
                Value::Primitive(Primitive::String(f)) => {
                    if f.trim() == "" {
                        false
                    } else {
                        true
                    }
                }
                _ => true,
            })
            .collect();

        let mut collected = Dictionary::default();
        collected.add(
            DataDescriptor::from(name.clone()),
            Value::List(children_values),
        );

        Value::Object(collected)
    } else if n.is_comment() {
        Value::Primitive(Primitive::String("<comment>".to_string()))
    } else if n.is_pi() {
        Value::Primitive(Primitive::String("<processing_instruction>".to_string()))
    } else if n.is_text() {
        Value::Primitive(Primitive::String(n.text().unwrap().to_string()))
    } else {
        Value::Primitive(Primitive::String("<unknown>".to_string()))
    }
}

fn from_document_to_value(d: &roxmltree::Document) -> Value {
    from_node_to_value(&d.root_element())
}

pub fn from_xml_string_to_value(s: String) -> Result<Value, Box<dyn std::error::Error>> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed))
}

pub fn from_xml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.name_span;
    Ok(out
        .map(move |a| match a {
            Value::Primitive(Primitive::String(s)) => match from_xml_string_to_value(s) {
                Ok(x) => ReturnValue::Value(x),
                Err(_) => {
                    ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                        "Could not parse as XML",
                        "piped data failed XML parse",
                        span,
                    ))))
                }
            },
            _ => ReturnValue::Value(Value::Error(Box::new(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )))),
        })
        .boxed())
}
