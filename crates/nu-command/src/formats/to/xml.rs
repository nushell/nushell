use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::IndexMap;
use nu_engine::command_prelude::*;

use quick_xml::{
    escape,
    events::{BytesEnd, BytesPI, BytesStart, BytesText, Event},
};
use std::{borrow::Cow, io::Cursor};

#[derive(Clone)]
pub struct ToXml;

impl Command for ToXml {
    fn name(&self) -> &str {
        "to xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to xml")
            .input_output_types(vec![(Type::record(), Type::String)])
            .named(
                "indent",
                SyntaxShape::Int,
                "Formats the XML text with the provided indentation setting",
                Some('i'),
            )
            .switch(
                "partial-escape",
                "Only escape mandatory characters in text and attributes",
                Some('p'),
            )
            .switch(
                "self-closed",
                "Output empty tags as self closing",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn extra_description(&self) -> &str {
        r#"Every XML entry is represented via a record with tag, attribute and content fields.
To represent different types of entries different values must be written to this fields:
1. Tag entry: `{tag: <tag name> attributes: {<attr name>: "<string value>" ...} content: [<entries>]}`
2. Comment entry: `{tag: '!' attributes: null content: "<comment string>"}`
3. Processing instruction (PI): `{tag: '?<pi name>' attributes: null content: "<pi content string>"}`
4. Text: `{tag: null attributes: null content: "<text>"}`. Or as plain `<text>` instead of record.

Additionally any field which is: empty record, empty list or null, can be omitted."#
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an XML string representing the contents of this table",
                example: r#"{tag: note attributes: {} content : [{tag: remember attributes: {} content : [{tag: null attributes: null content : Event}]}]} | to xml"#,
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
                example: r#"{tag: note content : [{tag: remember content : [Event]}]} | to xml --indent 3"#,
                result: Some(Value::test_string(
                    "<note>\n   <remember>Event</remember>\n</note>",
                )),
            },
            Example {
                description: "Produce less escaping sequences in resulting xml",
                example: r#"{tag: note attributes: {a: "'qwe'\\"} content: ["\"'"]} | to xml --partial-escape"#,
                result: Some(Value::test_string(r#"<note a="'qwe'\">"'</note>"#)),
            },
            Example {
                description: "Save space using self-closed tags",
                example: r#"{tag: root content: [[tag]; [a] [b] [c]]} | to xml --self-closed"#,
                result: Some(Value::test_string(r#"<root><a/><b/><c/></root>"#)),
            },
        ]
    }

    fn description(&self) -> &str {
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
        let indent: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "indent")?;
        let partial_escape = call.has_flag(engine_state, stack, "partial-escape")?;
        let self_closed = call.has_flag(engine_state, stack, "self-closed")?;

        let job = Job::new(indent, partial_escape, self_closed);
        let input = input.try_expand_range()?;
        job.run(input, head)
    }
}

struct Job {
    writer: quick_xml::Writer<Cursor<Vec<u8>>>,
    partial_escape: bool,
    self_closed: bool,
}

impl Job {
    fn new(indent: Option<Spanned<i64>>, partial_escape: bool, self_closed: bool) -> Self {
        let writer = indent.as_ref().map_or_else(
            || quick_xml::Writer::new(Cursor::new(Vec::new())),
            |p| quick_xml::Writer::new_with_indent(Cursor::new(Vec::new()), b' ', p.item as usize),
        );

        Self {
            writer,
            partial_escape,
            self_closed,
        }
    }

    fn run(mut self, input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
        let metadata = input
            .metadata()
            .unwrap_or_default()
            .with_content_type(Some("application/xml".into()));
        let value = input.into_value(head)?;

        self.write_xml_entry(value, true).and_then(|_| {
            let b = self.writer.into_inner().into_inner();
            let s = if let Ok(s) = String::from_utf8(b) {
                s
            } else {
                return Err(ShellError::NonUtf8 { span: head });
            };
            Ok(Value::string(s, head).into_pipeline_data_with_metadata(Some(metadata)))
        })
    }

    fn add_attributes<'a>(
        &self,
        element: &mut BytesStart<'a>,
        attributes: &'a IndexMap<String, String>,
    ) {
        for (k, v) in attributes {
            if self.partial_escape {
                element.push_attribute((k.as_bytes(), Self::partial_escape_attribute(v).as_ref()))
            } else {
                element.push_attribute((k.as_bytes(), escape::escape(v).as_bytes()))
            };
        }
    }

    fn partial_escape_attribute(raw: &str) -> Cow<[u8]> {
        let bytes = raw.as_bytes();
        let mut escaped: Vec<u8> = Vec::new();
        let mut iter = bytes.iter().enumerate();
        let mut pos = 0;
        while let Some((new_pos, byte)) =
            iter.find(|(_, ch)| matches!(ch, b'<' | b'>' | b'&' | b'"'))
        {
            escaped.extend_from_slice(&bytes[pos..new_pos]);
            match byte {
                b'<' => escaped.extend_from_slice(b"&lt;"),
                b'>' => escaped.extend_from_slice(b"&gt;"),
                b'&' => escaped.extend_from_slice(b"&amp;"),
                b'"' => escaped.extend_from_slice(b"&quot;"),

                _ => unreachable!("Only '<', '>','&', '\"' are escaped"),
            }
            pos = new_pos + 1;
        }

        if !escaped.is_empty() {
            if let Some(raw) = bytes.get(pos..) {
                escaped.extend_from_slice(raw);
            }

            Cow::Owned(escaped)
        } else {
            Cow::Borrowed(bytes)
        }
    }

    fn write_xml_entry(&mut self, entry: Value, top_level: bool) -> Result<(), ShellError> {
        let entry_span = entry.span();
        let span = entry.span();

        // Allow using strings directly as content.
        // So user can write
        // {tag: a content: ['qwe']}
        // instead of longer
        // {tag: a content: [{content: 'qwe'}]}
        if let (Value::String { val, .. }, false) = (&entry, top_level) {
            return self.write_xml_text(val.as_str(), span);
        }

        if let Value::Record { val: record, .. } = &entry {
            if let Some(bad_column) = Self::find_invalid_column(record) {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: "record".into(),
                    span: entry_span,
                    help: Some(format!(
                        "Invalid column \"{bad_column}\" in xml entry. Only \"{COLUMN_TAG_NAME}\", \"{COLUMN_ATTRS_NAME}\" and \"{COLUMN_CONTENT_NAME}\" are permitted"
                    )),
                });
            }
            // If key is not found it is assumed to be nothing. This way
            // user can write a tag like {tag: a content: [...]} instead
            // of longer {tag: a attributes: {} content: [...]}
            let tag = record
                .get(COLUMN_TAG_NAME)
                .cloned()
                .unwrap_or_else(|| Value::nothing(Span::unknown()));
            let attrs = record
                .get(COLUMN_ATTRS_NAME)
                .cloned()
                .unwrap_or_else(|| Value::nothing(Span::unknown()));
            let content = record
                .get(COLUMN_CONTENT_NAME)
                .cloned()
                .unwrap_or_else(|| Value::nothing(Span::unknown()));

            let content_span = content.span();
            let tag_span = tag.span();
            match (tag, attrs, content) {
                (Value::Nothing { .. }, Value::Nothing { .. }, Value::String { val, .. }) => {
                    // Strings can not appear on top level of document
                    if top_level {
                        return Err(ShellError::CantConvert {
                            to_type: "XML".into(),
                            from_type: entry.get_type().to_string(),
                            span: entry_span,
                            help: Some("Strings can not be a root element of document".into()),
                        });
                    }
                    self.write_xml_text(val.as_str(), content_span)
                }
                (Value::String { val: tag_name, .. }, attrs, children) => {
                    self.write_tag_like(entry_span, tag_name, tag_span, attrs, children, top_level)
                }
                _ => Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: "record".into(),
                    span: entry_span,
                    help: Some("Tag missing or is not a string".into()),
                }),
            }
        } else {
            Err(ShellError::CantConvert {
                to_type: "XML".into(),
                from_type: entry.get_type().to_string(),
                span: entry_span,
                help: Some("Xml entry expected to be a record".into()),
            })
        }
    }

    fn find_invalid_column(record: &Record) -> Option<&String> {
        const VALID_COLS: [&str; 3] = [COLUMN_TAG_NAME, COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME];
        record
            .columns()
            .find(|col| !VALID_COLS.contains(&col.as_str()))
    }

    /// Convert record to tag-like entry: tag, PI, comment.
    fn write_tag_like(
        &mut self,
        entry_span: Span,
        tag: String,
        tag_span: Span,
        attrs: Value,
        content: Value,
        top_level: bool,
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

            self.write_comment(entry_span, attrs, content)
        } else if let Some(tag) = tag.strip_prefix('?') {
            // PIs can not appear on top level of document
            if top_level {
                return Err(ShellError::CantConvert {
                    to_type: "XML".into(),
                    from_type: Type::record().to_string(),
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
                        from_type: Type::record().to_string(),
                        span: content.span(),
                        help: Some("PI content expected to be a string".into()),
                    });
                }
            };

            self.write_processing_instruction(entry_span, tag, attrs, content)
        } else {
            // Allow tag to have no attributes or content for short hand input
            // alternatives like {tag: a attributes: {} content: []}, {tag: a attribbutes: null
            // content: null}, {tag: a}. See to_xml_entry for more
            let attrs = match attrs {
                Value::Record { val, .. } => val.into_owned(),
                Value::Nothing { .. } => Record::new(),
                _ => {
                    return Err(ShellError::CantConvert {
                        to_type: "XML".into(),
                        from_type: attrs.get_type().to_string(),
                        span: attrs.span(),
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
                        span: content.span(),
                        help: Some("Tag content expected to be a list".into()),
                    });
                }
            };

            self.write_tag(entry_span, tag, tag_span, attrs, content)
        }
    }

    fn write_comment(
        &mut self,
        entry_span: Span,
        attrs: Value,
        content: Value,
    ) -> Result<(), ShellError> {
        match (attrs, content) {
            (Value::Nothing { .. }, Value::String { val, .. }) => {
                // Text in comments must NOT be escaped
                // https://www.w3.org/TR/xml/#sec-comments
                let comment_content = BytesText::from_escaped(val.as_str());
                self.writer
                    .write_event(Event::Comment(comment_content))
                    .map_err(|_| ShellError::CantConvert {
                        to_type: "XML".to_string(),
                        from_type: Type::record().to_string(),
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

    fn write_processing_instruction(
        &mut self,
        entry_span: Span,
        tag: &str,
        attrs: Value,
        content: String,
    ) -> Result<(), ShellError> {
        if !matches!(attrs, Value::Nothing { .. }) {
            return Err(ShellError::CantConvert {
                to_type: "XML".into(),
                from_type: Type::record().to_string(),
                span: entry_span,
                help: Some("PIs do not have attributes".into()),
            });
        }

        let content_text = format!("{tag} {content}");
        // PI content must NOT be escaped
        // https://www.w3.org/TR/xml/#sec-pi
        let pi_content = BytesPI::new(content_text.as_str());

        self.writer
            .write_event(Event::PI(pi_content))
            .map_err(|_| ShellError::CantConvert {
                to_type: "XML".to_string(),
                from_type: Type::record().to_string(),
                span: entry_span,
                help: Some("Failure writing PI to xml".into()),
            })
    }

    fn write_tag(
        &mut self,
        entry_span: Span,
        tag: String,
        tag_span: Span,
        attrs: Record,
        children: Vec<Value>,
    ) -> Result<(), ShellError> {
        if tag.starts_with('!') || tag.starts_with('?') {
            return Err(ShellError::CantConvert {
                to_type: "XML".to_string(),
                from_type: Type::record().to_string(),
                span: tag_span,
                help: Some(format!(
                    "Incorrect tag name {tag}, tag name can not start with ! or ?"
                )),
            });
        }

        let self_closed = self.self_closed && children.is_empty();
        let attributes = Self::parse_attributes(attrs)?;
        let mut open_tag = BytesStart::new(tag.clone());
        self.add_attributes(&mut open_tag, &attributes);
        let open_tag_event = if self_closed {
            Event::Empty(open_tag)
        } else {
            Event::Start(open_tag)
        };

        self.writer
            .write_event(open_tag_event)
            .map_err(|_| ShellError::CantConvert {
                to_type: "XML".to_string(),
                from_type: Type::record().to_string(),
                span: entry_span,
                help: Some("Failure writing tag to xml".into()),
            })?;

        children
            .into_iter()
            .try_for_each(|child| self.write_xml_entry(child, false))?;

        if !self_closed {
            let close_tag_event = Event::End(BytesEnd::new(tag));
            self.writer
                .write_event(close_tag_event)
                .map_err(|_| ShellError::CantConvert {
                    to_type: "XML".to_string(),
                    from_type: Type::record().to_string(),
                    span: entry_span,
                    help: Some("Failure writing tag to xml".into()),
                })?;
        }
        Ok(())
    }

    fn parse_attributes(attrs: Record) -> Result<IndexMap<String, String>, ShellError> {
        let mut h = IndexMap::new();
        for (k, v) in attrs {
            if let Value::String { val, .. } = v {
                h.insert(k, val);
            } else {
                return Err(ShellError::CantConvert {
                    to_type: "XML".to_string(),
                    from_type: v.get_type().to_string(),
                    span: v.span(),
                    help: Some("Attribute value expected to be a string".into()),
                });
            }
        }
        Ok(h)
    }

    fn write_xml_text(&mut self, val: &str, span: Span) -> Result<(), ShellError> {
        let text = Event::Text(if self.partial_escape {
            BytesText::from_escaped(escape::partial_escape(val))
        } else {
            BytesText::new(val)
        });

        self.writer
            .write_event(text)
            .map_err(|_| ShellError::CantConvert {
                to_type: "XML".to_string(),
                from_type: Type::String.to_string(),
                span,
                help: Some("Failure writing string to xml".into()),
            })
    }
}

#[cfg(test)]
mod test {
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    use crate::{Get, Metadata};

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToXml {})
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToXml {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{tag: note attributes: {} content : [{tag: remember attributes: {} content : [{tag: null attributes: null content : Event}]}]} | to xml | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("application/xml"),
            result.expect("There should be a result")
        );
    }
}
