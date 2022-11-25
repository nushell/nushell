use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use std::collections::HashSet;
use std::io::Cursor;
use std::io::Write;

#[derive(Clone)]
pub struct ToXml;

impl Command for ToXml {
    fn name(&self) -> &str {
        "to xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to xml")
            .input_output_types(vec![(Type::Record(vec![]), Type::String)])
            .named(
                "pretty",
                SyntaxShape::Int,
                "Formats the XML text with the provided indentation setting",
                Some('p'),
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an XML string representing the contents of this table",
                example: r#"{ "note": { "children": [{ "remember": {"attributes" : {}, "children": [Event]}}], "attributes": {} } } | to xml"#,
                result: Some(Value::test_string(
                    "<note><remember>Event</remember></note>",
                )),
            },
            Example {
                description: "Optionally, formats the text with a custom indentation setting",
                example: r#"{ "note": { "children": [{ "remember": {"attributes" : {}, "children": [Event]}}], "attributes": {} } } | to xml -p 3"#,
                result: Some(Value::test_string(
                    "<note>\n   <remember>Event</remember>\n</note>",
                )),
            },
        ]
    }

    fn usage(&self) -> &str {
        "Convert table into .xml text"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = engine_state.get_config();
        let pretty: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "pretty")?;
        to_xml(input, head, pretty, config)
    }
}

pub fn add_attributes<'a>(
    element: &mut quick_xml::events::BytesStart<'a>,
    attributes: &'a IndexMap<String, String>,
) {
    for (k, v) in attributes {
        element.push_attribute((k.as_str(), v.as_str()));
    }
}

pub fn get_attributes(row: &Value, config: &Config) -> Option<IndexMap<String, String>> {
    if let Value::Record { .. } = row {
        if let Some(Value::Record { cols, vals, .. }) = row.get_data_by_key("attributes") {
            let mut h = IndexMap::new();
            for (k, v) in cols.iter().zip(vals.iter()) {
                h.insert(k.clone(), v.clone().into_abbreviated_string(config));
            }
            return Some(h);
        }
    }
    None
}

pub fn get_children(row: &Value) -> Option<Vec<Value>> {
    if let Value::Record { .. } = row {
        if let Some(Value::List { vals, .. }) = row.get_data_by_key("children") {
            return Some(vals);
        }
    }
    None
}

pub fn is_xml_row(row: &Value) -> bool {
    if let Value::Record { cols, .. } = &row {
        let keys: HashSet<&String> = cols.iter().collect();
        let children: String = "children".to_string();
        let attributes: String = "attributes".to_string();
        return keys.contains(&children) && keys.contains(&attributes) && keys.len() == 2;
    }
    false
}

pub fn write_xml_events<W: Write>(
    current: Value,
    writer: &mut quick_xml::Writer<W>,
    config: &Config,
) -> Result<(), ShellError> {
    match current {
        Value::Record { cols, vals, span } => {
            for (k, v) in cols.iter().zip(vals.iter()) {
                let mut e = BytesStart::owned(k.as_bytes(), k.len());
                if !is_xml_row(v) {
                    return Err(ShellError::GenericError(
                        "Expected a row with 'children' and 'attributes' columns".to_string(),
                        "missing 'children' and 'attributes' columns ".to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    ));
                }
                let a = get_attributes(v, config);
                if let Some(ref a) = a {
                    add_attributes(&mut e, a);
                }
                writer
                    .write_event(Event::Start(e))
                    .expect("Couldn't open XML node");
                let c = get_children(v);
                if let Some(c) = c {
                    for v in c {
                        write_xml_events(v, writer, config)?;
                    }
                }
                writer
                    .write_event(Event::End(BytesEnd::borrowed(k.as_bytes())))
                    .expect("Couldn't close XML node");
            }
        }
        Value::List { vals, .. } => {
            for v in vals {
                write_xml_events(v, writer, config)?;
            }
        }
        _ => {
            let s = current.into_abbreviated_string(config);
            writer
                .write_event(Event::Text(BytesText::from_plain_str(s.as_str())))
                .expect("Couldn't write XML text");
        }
    }
    Ok(())
}

fn to_xml(
    input: PipelineData,
    head: Span,
    pretty: Option<Spanned<i64>>,
    config: &Config,
) -> Result<PipelineData, ShellError> {
    let mut w = pretty.as_ref().map_or_else(
        || quick_xml::Writer::new(Cursor::new(Vec::new())),
        |p| quick_xml::Writer::new_with_indent(Cursor::new(Vec::new()), b' ', p.item as usize),
    );

    let value = input.into_value(head);
    let value_type = value.get_type();

    match write_xml_events(value, &mut w, config) {
        Ok(_) => {
            let b = w.into_inner().into_inner();
            let s = if let Ok(s) = String::from_utf8(b) {
                s
            } else {
                return Err(ShellError::NonUtf8(head));
            };
            Ok(Value::string(s, head).into_pipeline_data())
        }
        Err(_) => Err(ShellError::CantConvert(
            "XML".into(),
            value_type.to_string(),
            head,
            None,
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToXml {})
    }
}
