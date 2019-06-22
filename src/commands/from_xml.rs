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
        Value::string("<comment>")
    } else if n.is_pi() {
        Value::string("<processing_instruction>")
    } else if n.is_text() {
        Value::string(n.text().unwrap())
    } else {
        Value::string("<unknown>")
    }
}

fn from_document_to_value(d: &roxmltree::Document) -> Value {
    from_node_to_value(&d.root_element())
}

pub fn from_xml_string_to_value(s: impl AsRef<str>) -> Value {
    match roxmltree::Document::parse(s.as_ref()) {
        Ok(doc) => from_document_to_value(&doc),
        Err(_) => Value::Error(Box::new(ShellError::string(
            "Can't convert string from xml".to_string(),
        ))),
    }
}

pub fn from_xml(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    Ok(out
        .map(|a| match a {
            Value::Primitive(Primitive::String(s)) => {
                ReturnValue::Value(from_xml_string_to_value(s))
            }
            _ => ReturnValue::Value(Value::Error(Box::new(ShellError::string(
                "Trying to convert XML from non-string".to_string(),
            )))),
        })
        .boxed())
}
