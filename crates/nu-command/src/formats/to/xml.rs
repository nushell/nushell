use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
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

    fn extra_usage(&self) -> &str {
        r#"Every XML entry is represented via a record with tag, attribute and content fields.
To represent different types of entries different values must be written to this fields:
1. Tag entry: `{tag: <tag name> attrs: {<attr name>: "<string value>" ...} content: [<entries>]}`
2. Comment entry: `{tag: '!' attrs: null content: "<comment string>"}`
3. Processing instruction (PI): `{tag: '?<pi name>' attrs: null content: "<pi content string>"}`
4. Text: `{tag: null attrs: null content: "<text>"}`. Or as plain "<text>" instead of record.

Additionally any field which is: empty record, empty list or null, can be omitted."#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an XML string representing the contents of this table",
                example: r#"{tag: note attributes: {} content : [{tag: remember attributes: {} content : [{tag: null attrs: null content : Event}]}]} | to xml"#,
                result: Some(Value::test_string(
                    "<note><remember>Event</remember></note>",
                )),
            },
            Example {
                description: "When formatting xml null and empty record fields can be omitted and strings can be written without a wrapping record",
                example: r#"{tag: note content : [{tag: remember content : [Event]}]} | to xml"#,
                result: Some(Value::test_string(
                    "<note><remember>Event</remember></note>",
                )),
            },
            Example {
                description: "Optionally, formats the text with a custom indentation setting",
                example: r#"{tag: note content : [{tag: remember content : [Event]}]} | to xml -p 3"#,
                result: Some(Value::test_string(
                    "<note>\n   <remember>Event</remember>\n</note>",
                )),
            },
        ]
    }

    fn usage(&self) -> &str {
        "Convert special record structure into .xml text."
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
        let input = input.try_expand_range()?;
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
    top_level: bool,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    let entry_span = entry.span()?;

    // Allow using strings directly as content.
    // So user can write
    // {tag: a content: ['qwe']}
    // instead of longer
    // {tag: a content: [{content: 'qwe'}]}
    if let (Value::String { val, span }, false) = (&entry, top_level) {
        return to_xml_text(val.as_str(), *span, writer);
    }

    if !matches!(entry, Value::Record { .. }) {
        return Err(ShellError::CantConvert {
            to_type: "XML".into(),
            from_type: entry.get_type().to_string(),
            span: entry_span,
            help: Some("Xml entry expected to be a record".into()),
        });
    };

    // If key is not found it is assumed to be nothing. This way
    // user can write a tag like {tag: a content: [...]} instead
    // of longer {tag: a attributes: {} content: [...]}
    let tag = entry
        .get_data_by_key(COLUMN_TAG_NAME)
        .unwrap_or_else(|| Value::nothing(Span::unknown()));
    let attrs = entry
        .get_data_by_key(COLUMN_ATTRS_NAME)
        .unwrap_or_else(|| Value::nothing(Span::unknown()));
    let content = entry
        .get_data_by_key(COLUMN_CONTENT_NAME)
        .unwrap_or_else(|| Value::nothing(Span::unknown()));

    match (tag, attrs, content) {
        (Value::Nothing { .. }, Value::Nothing { .. }, Value::String { val, span }) => {
            // Strings can not appear on top level of document
            if top_level {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: entry.get_type().to_string(),
                    span: entry_span,
                    help: Some("Strings can not be a root element of document".into()),
                });
            }
            to_xml_text(val.as_str(), span, writer)
        }
        (
            Value::String {
                val: tag_name,
                span: tag_span,
            },
            attrs,
            children,
        ) => to_tag_like(
            entry_span, tag_name, tag_span, attrs, children, top_level, writer,
        ),
        _ => Ok(()),
    }
}

/// Convert record to tag-like entry: tag, PI, comment.
fn to_tag_like<W: Write>(
    entry_span: Span,
    tag: String,
    tag_span: Span,
    attrs: Value,
    content: Value,
    top_level: bool,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    if tag == "!" {
        // Comments can not appear on top level of document
        if top_level {
            return Err(ShellError::CantConvert {
                to_type: "XML".into(),
                from_type: "record".into(),
                span: entry_span,
                help: Some("Comments can not be a root element of document".into()),
            });
        }

        to_comment(entry_span, attrs, content, writer)
    } else if let Some(tag) = tag.strip_prefix('?') {
        // PIs can not appear on top level of document
        if top_level {
            return Err(ShellError::CantConvert {
                to_type: "XML".into(),
                from_type: Type::Record(vec![]).to_string(),
                span: entry_span,
                help: Some("PIs can not be a root element of document".into()),
            });
        }

        let content: String = match content {
            Value::String { val, .. } => val,
            Value::Nothing { .. } => "".into(),
            _ => {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: Type::Record(vec![]).to_string(),
                    span: content.span()?,
                    help: Some("PI content expected to be a string".into()),
                });
            }
        };

        to_processing_instruction(entry_span, tag, attrs, content, writer)
    } else {
        // Allow tag to have no attributes or content for short hand input
        // alternatives like {tag: a attributes: {} content: []}, {tag: a attribbutes: null
        // content: null}, {tag: a}. See to_xml_entry for more
        let (attr_cols, attr_values) = match attrs {
            Value::Record { cols, vals, .. } => (cols, vals),
            Value::Nothing { .. } => (Vec::new(), Vec::new()),
            _ => {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: attrs.get_type().to_string(),
                    span: attrs.span()?,
                    help: Some("Tag attributes expected to be a record".into()),
                });
            }
        };

        let content = match content {
            Value::List { vals, .. } => vals,
            Value::Nothing { .. } => Vec::new(),
            _ => {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: content.get_type().to_string(),
                    span: content.span()?,
                    help: Some("Tag content expected to be a list".into()),
                });
            }
        };

        to_tag(
            entry_span,
            tag,
            tag_span,
            attr_cols,
            attr_values,
            content,
            writer,
        )
    }
}

fn to_comment<W: Write>(
    entry_span: Span,
    attrs: Value,
    content: Value,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    match (attrs, content) {
        (Value::Nothing { .. }, Value::String { val, .. }) => {
            let comment_content = BytesText::new(val.as_str());
            writer
                .write_event(Event::Comment(comment_content))
                .map_err(|_| ShellError::CantConvert {
                    to_type: "XML".to_string(),
                    from_type: Type::Record(vec![]).to_string(),
                    span: entry_span,
                    help: Some("Failure writing comment to xml".into()),
                })
        }
        (_, content) => Err(ShellError::CantConvert {
            to_type: "XML".into(),
            from_type: content.get_type().to_string(),
            span: entry_span,
            help: Some("Comment expected to have string content and no attributes".into()),
        }),
    }
}

fn to_processing_instruction<W: Write>(
    entry_span: Span,
    tag: &str,
    attrs: Value,
    content: String,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    if !matches!(attrs, Value::Nothing { .. }) {
        return Err(ShellError::CantConvert {
            to_type: "XML".into(),
            from_type: Type::Record(vec![]).to_string(),
            span: entry_span,
            help: Some("PIs do not have attributes".into()),
        });
    }

    let content_text = format!("{} {}", tag, content);
    let pi_content = BytesText::new(content_text.as_str());
    writer
        .write_event(Event::PI(pi_content))
        .map_err(|_| ShellError::CantConvert {
            to_type: "XML".to_string(),
            from_type: Type::Record(vec![]).to_string(),
            span: entry_span,
            help: Some("Failure writing PI to xml".into()),
        })
}

fn to_tag<W: Write>(
    entry_span: Span,
    tag: String,
    tag_span: Span,
    attr_cols: Vec<String>,
    attr_vals: Vec<Value>,
    children: Vec<Value>,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    if tag.starts_with('!') || tag.starts_with('?') {
        return Err(ShellError::CantConvert {
            to_type: "XML".to_string(),
            from_type: Type::Record(vec![]).to_string(),
            span: tag_span,
            help: Some(format!(
                "Incorrect tag name {}, tag name can not start with ! or ?",
                tag
            )),
        });
    }

    let attributes = parse_attributes(attr_cols, attr_vals)?;
    let mut open_tag_event = BytesStart::new(tag.clone());
    add_attributes(&mut open_tag_event, &attributes);

    writer
        .write_event(Event::Start(open_tag_event))
        .map_err(|_| ShellError::CantConvert {
            to_type: "XML".to_string(),
            from_type: Type::Record(vec![]).to_string(),
            span: entry_span,
            help: Some("Failure writing tag to xml".into()),
        })?;

    children
        .into_iter()
        .try_for_each(|child| to_xml_entry(child, false, writer))?;

    let close_tag_event = BytesEnd::new(tag);
    writer
        .write_event(Event::End(close_tag_event))
        .map_err(|_| ShellError::CantConvert {
            to_type: "XML".to_string(),
            from_type: Type::Record(vec![]).to_string(),
            span: entry_span,
            help: Some("Failure writing tag to xml".into()),
        })
}

fn parse_attributes(
    cols: Vec<String>,
    vals: Vec<Value>,
) -> Result<IndexMap<String, String>, ShellError> {
    let mut h = IndexMap::new();
    for (k, v) in cols.into_iter().zip(vals.into_iter()) {
        if let Value::String { val, .. } = v {
            h.insert(k, val);
        } else {
            return Err(ShellError::CantConvert {
                to_type: "XML".to_string(),
                from_type: v.get_type().to_string(),
                span: v.span()?,
                help: Some("Attribute value expected to be a string".into()),
            });
        }
    }
    Ok(h)
}

fn to_xml_text<W: Write>(
    val: &str,
    span: Span,
    writer: &mut quick_xml::Writer<W>,
) -> Result<(), ShellError> {
    let text = Event::Text(BytesText::new(val));
    writer
        .write_event(text)
        .map_err(|_| ShellError::CantConvert {
            to_type: "XML".to_string(),
            from_type: Type::String.to_string(),
            span,
            help: Some("Failure writing string to xml".into()),
        })
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

    to_xml_entry(value, true, &mut w).and_then(|_| {
        let b = w.into_inner().into_inner();
        let s = if let Ok(s) = String::from_utf8(b) {
            s
        } else {
            return Err(ShellError::NonUtf8(head));
        };
        Ok(Value::string(s, head).into_pipeline_data())
    })
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
