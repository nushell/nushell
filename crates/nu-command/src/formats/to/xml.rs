use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let pretty: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "pretty")?;
        to_xml(input, head, pretty)
    }
}

pub fn add_attributes<'a>(element: &mut BytesStart<'a>, attributes: &'a IndexMap<String, String>) {
    for (k, v) in attributes {
        element.push_attribute((k.as_str(), v.as_str()));
    }
}

fn to_xml_entry<W: Write>(
    entry: Value,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    if !matches!(entry, Value::Record { .. }) {
        return Err(ShellError::CantConvert(
            "XML".into(),
            entry.get_type().to_string(),
            entry.span()?,
            None,
        ));
    };

    let tag = entry.get_data_by_key(COLUMN_TAG_NAME).ok_or(ShellError::CantConvert(
        "XML".into(),
        entry.get_type().to_string(),
        entry.span()?,
        None,
    ))?;
    let attrs = entry.get_data_by_key(COLUMN_ATTRS_NAME).ok_or(ShellError::CantConvert(
        "XML".into(),
        entry.get_type().to_string(),
        entry.span()?,
        None,
    ))?;
    let content = entry.get_data_by_key(COLUMN_CONTENT_NAME).ok_or(ShellError::CantConvert(
        "XML".into(),
        entry.get_type().to_string(),
        entry.span()?,
        None,
    ))?;

    match (tag, attrs, content) {
        (Value::Nothing { .. }, Value::Nothing { .. }, Value::String { val, .. }) => {
            to_xml_text(val, writer)
        }
        (
            Value::String { val: tag_name, .. },
            Value::Record {
                cols: attr_cols,
                vals: attr_vals,
                ..
            },
            Value::List { vals: children, .. },
        ) => {to_tag(tag_name, attr_cols, attr_vals, children, writer)}
        _ => Ok(()),
    }
    .map_err(|_| {
        ShellError::CantConvert(
            "XML".into(),
            entry.get_type().to_string(),
            entry.span().unwrap_or(Span::unknown()),
            None,
        )
    })?;

    Ok(())
}

/// Convert record to tag-like entry: tag, PI, comment.
fn to_tag<W: Write>(
    tag: String,
    attr_cols: Vec<String>,
    attr_vals: Vec<Value>,
    children: Vec<Value>,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ()> {
    if tag.starts_with('!') || tag.starts_with('?') {
        return Err(());
    }

    let attributes = parse_attributes(attr_cols, attr_vals)?;
    let mut open_tag_event = BytesStart::new(tag.clone());
    add_attributes(&mut open_tag_event, &attributes);

    writer.write_event(Event::Start(open_tag_event)).map_err(|_| ())?;

    children.into_iter()
        .try_for_each(|child| to_xml_entry(child, writer)).map_err(|_| ())?;

    let close_tag_event = BytesEnd::new(tag);
    writer.write_event(Event::End(close_tag_event)).map_err(|_| ())
}

fn parse_attributes(
    cols: Vec<String>,
    vals: Vec<Value>) -> Result<IndexMap<String, String>, ()> {
    let mut h = IndexMap::new();
    for (k, v) in cols.into_iter().zip(vals.into_iter()) {
        if let Value::String {val, ..} = v {
            h.insert(k, val);
        } else {
            return Err(());
        }
    }
    Ok(h)
}

fn to_xml_text<W: Write>(val: String, writer: &mut quick_xml::Writer<W>) -> Result<(), ()> {
    let text = Event::Text(BytesText::new(val.as_str()));
    writer.write_event(text).map_err(|_| ())?;
    Ok(())
}

fn to_xml(
    input: PipelineData,
    head: Span,
    pretty: Option<Spanned<i64>>,
) -> Result<PipelineData, ShellError> {
    let mut w = pretty.as_ref().map_or_else(
        || quick_xml::Writer::new(Cursor::new(Vec::new())),
        |p| quick_xml::Writer::new_with_indent(Cursor::new(Vec::new()), b' ', p.item as usize),
    );

    let value = input.into_value(head);
    let value_type = value.get_type();

    match to_xml_entry(value, &mut w) {
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
