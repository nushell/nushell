use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::IndexMap;
use nu_engine::command_prelude::*;

use roxmltree::NodeType;

#[derive(Clone)]
pub struct FromXml;

impl Command for FromXml {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
            .input_output_types(vec![(Type::String, Type::record())])
            .switch("keep-comments", "add comment nodes to result", None)
            .switch(
                "keep-pi",
                "add processing instruction nodes to result",
                None,
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .xml and create record."
    }

    fn extra_usage(&self) -> &str {
        r#"Every XML entry is represented via a record with tag, attribute and content fields.
To represent different types of entries different values are written to this fields:
1. Tag entry: `{tag: <tag name> attrs: {<attr name>: "<string value>" ...} content: [<entries>]}`
2. Comment entry: `{tag: '!' attrs: null content: "<comment string>"}`
3. Processing instruction (PI): `{tag: '?<pi name>' attrs: null content: "<pi content string>"}`
4. Text: `{tag: null attrs: null content: "<text>"}`.

Unlike to xml command all null values are always present and text is never represented via plain
string. This way content of every tag is always a table and is easier to parse"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let keep_comments = call.has_flag(engine_state, stack, "keep-comments")?;
        let keep_processing_instructions = call.has_flag(engine_state, stack, "keep-pi")?;
        let info = ParsingInfo {
            span: head,
            keep_comments,
            keep_processing_instructions,
        };
        from_xml(input, &info)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: r#"'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml"#,
            description: "Converts xml formatted string to record",
            result: Some(Value::test_record(record! {
                COLUMN_TAG_NAME =>     Value::test_string("note"),
                COLUMN_ATTRS_NAME =>   Value::test_record(Record::new()),
                COLUMN_CONTENT_NAME => Value::test_list(vec![
                Value::test_record(record! {
                    COLUMN_TAG_NAME =>     Value::test_string("remember"),
                    COLUMN_ATTRS_NAME =>   Value::test_record(Record::new()),
                    COLUMN_CONTENT_NAME => Value::test_list(vec![
                    Value::test_record(record! {
                        COLUMN_TAG_NAME =>     Value::test_nothing(),
                        COLUMN_ATTRS_NAME =>   Value::test_nothing(),
                        COLUMN_CONTENT_NAME => Value::test_string("Event"),
                        })],
                    ),
                    })],
                ),
            })),
        }]
    }
}

struct ParsingInfo {
    span: Span,
    keep_comments: bool,
    keep_processing_instructions: bool,
}

fn from_attributes_to_value(attributes: &[roxmltree::Attribute], info: &ParsingInfo) -> Value {
    let mut collected = IndexMap::new();
    for a in attributes {
        collected.insert(String::from(a.name()), Value::string(a.value(), info.span));
    }
    Value::record(collected.into_iter().collect(), info.span)
}

fn element_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Value {
    let span = info.span;
    let mut node = IndexMap::new();

    let tag = n.tag_name().name().trim().to_string();
    let tag = Value::string(tag, span);

    let content: Vec<Value> = n
        .children()
        .filter_map(|node| from_node_to_value(&node, info))
        .collect();
    let content = Value::list(content, span);

    let attributes = from_attributes_to_value(&n.attributes().collect::<Vec<_>>(), info);

    node.insert(String::from(COLUMN_TAG_NAME), tag);
    node.insert(String::from(COLUMN_ATTRS_NAME), attributes);
    node.insert(String::from(COLUMN_CONTENT_NAME), content);

    Value::record(node.into_iter().collect(), span)
}

fn text_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    let span = info.span;
    let text = n.text().expect("Non-text node supplied to text_to_value");
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        let mut node = IndexMap::new();
        let content = Value::string(String::from(text), span);

        node.insert(String::from(COLUMN_TAG_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        Some(Value::record(node.into_iter().collect(), span))
    }
}

fn comment_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    if info.keep_comments {
        let span = info.span;
        let text = n
            .text()
            .expect("Non-comment node supplied to comment_to_value");

        let mut node = IndexMap::new();
        let content = Value::string(String::from(text), span);

        node.insert(String::from(COLUMN_TAG_NAME), Value::string("!", span));
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        Some(Value::record(node.into_iter().collect(), span))
    } else {
        None
    }
}

fn processing_instruction_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    if info.keep_processing_instructions {
        let span = info.span;
        let pi = n.pi()?;

        let mut node = IndexMap::new();
        // Add '?' before target to differentiate tags from pi targets
        let tag = format!("?{}", pi.target);
        let tag = Value::string(tag, span);
        let content = pi
            .value
            .map_or_else(|| Value::nothing(span), |x| Value::string(x, span));

        node.insert(String::from(COLUMN_TAG_NAME), tag);
        node.insert(String::from(COLUMN_ATTRS_NAME), Value::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        Some(Value::record(node.into_iter().collect(), span))
    } else {
        None
    }
}

fn from_node_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<Value> {
    match n.node_type() {
        NodeType::Element => Some(element_to_value(n, info)),
        NodeType::Text => text_to_value(n, info),
        NodeType::Comment => comment_to_value(n, info),
        NodeType::PI => processing_instruction_to_value(n, info),
        _ => None,
    }
}

fn from_document_to_value(d: &roxmltree::Document, info: &ParsingInfo) -> Value {
    element_to_value(&d.root_element(), info)
}

fn from_xml_string_to_value(s: String, info: &ParsingInfo) -> Result<Value, roxmltree::Error> {
    let parsed = roxmltree::Document::parse(&s)?;
    Ok(from_document_to_value(&parsed, info))
}

fn from_xml(input: PipelineData, info: &ParsingInfo) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(info.span)?;

    match from_xml_string_to_value(concat_string, info) {
        Ok(x) => Ok(x.into_pipeline_data_with_metadata(metadata)),
        Err(err) => Err(process_xml_parse_error(err, span)),
    }
}

fn process_xml_parse_error(err: roxmltree::Error, span: Span) -> ShellError {
    match err {
        roxmltree::Error::InvalidXmlPrefixUri(_) => make_cant_convert_error(
            "The `xmlns:xml` attribute must have an <http://www.w3.org/XML/1998/namespace> URI.",
            span,
        ),
        roxmltree::Error::UnexpectedXmlUri(_) => make_cant_convert_error(
            "Only the xmlns:xml attribute can have the http://www.w3.org/XML/1998/namespace  URI.",
            span,
        ),
        roxmltree::Error::UnexpectedXmlnsUri(_) => make_cant_convert_error(
            "The http://www.w3.org/2000/xmlns/  URI must not be declared.",
            span,
        ),
        roxmltree::Error::InvalidElementNamePrefix(_) => {
            make_cant_convert_error("xmlns can't be used as an element prefix.", span)
        }
        roxmltree::Error::DuplicatedNamespace(_, _) => {
            make_cant_convert_error("A namespace was already defined on this element.", span)
        }
        roxmltree::Error::UnknownNamespace(prefix, _) => {
            make_cant_convert_error(format!("Unknown prefix {}", prefix), span)
        }
        roxmltree::Error::UnexpectedCloseTag { .. } => {
            make_cant_convert_error("Unexpected close tag", span)
        }
        roxmltree::Error::UnexpectedEntityCloseTag(_) => {
            make_cant_convert_error("Entity value starts with a close tag.", span)
        }
        roxmltree::Error::UnknownEntityReference(_, _) => make_cant_convert_error(
            "A reference to an entity that was not defined in the DTD.",
            span,
        ),
        roxmltree::Error::MalformedEntityReference(_) => {
            make_cant_convert_error("A malformed entity reference.", span)
        }
        roxmltree::Error::EntityReferenceLoop(_) => {
            make_cant_convert_error("A possible entity reference loop.", span)
        }
        roxmltree::Error::InvalidAttributeValue(_) => {
            make_cant_convert_error("Attribute value cannot have a < character.", span)
        }
        roxmltree::Error::DuplicatedAttribute(_, _) => {
            make_cant_convert_error("An element has a duplicated attributes.", span)
        }
        roxmltree::Error::NoRootNode => {
            make_cant_convert_error("The XML document must have at least one element.", span)
        }
        roxmltree::Error::UnclosedRootNode => {
            make_cant_convert_error("The root node was opened but never closed.", span)
        }
        roxmltree::Error::DtdDetected => make_cant_convert_error(
            "An XML with DTD detected. DTDs are currently disabled due to security reasons.",
            span,
        ),
        roxmltree::Error::NodesLimitReached => {
            make_cant_convert_error("Node limit was reached.", span)
        }
        roxmltree::Error::AttributesLimitReached => {
            make_cant_convert_error("Attribute limit reached", span)
        }
        roxmltree::Error::NamespacesLimitReached => {
            make_cant_convert_error("Namespace limit reached", span)
        }
        roxmltree::Error::UnexpectedDeclaration(_) => {
            make_cant_convert_error("An XML document can have only one XML declaration and it must be at the start of the document.", span)
        }
        roxmltree::Error::InvalidName(_) => {
            make_cant_convert_error("Invalid name found.", span)
        }
        roxmltree::Error::NonXmlChar(_, _) => {
            make_cant_convert_error("A non-XML character has occurred. Valid characters are: <https://www.w3.org/TR/xml/#char32>", span)
        }
        roxmltree::Error::InvalidChar(_, _, _) => {
            make_cant_convert_error("An invalid/unexpected character in XML.", span)
        }
        roxmltree::Error::InvalidChar2(_, _, _) => {
            make_cant_convert_error("An invalid/unexpected character in XML.", span)
        }
        roxmltree::Error::InvalidString(_, _) => {
            make_cant_convert_error("An invalid/unexpected string in XML.", span)
        }
        roxmltree::Error::InvalidExternalID(_) => {
            make_cant_convert_error("An invalid ExternalID in the DTD.", span)
        }
        roxmltree::Error::InvalidComment(_) => {
            make_cant_convert_error("A comment cannot contain `--` or end with `-`.", span)
        }
        roxmltree::Error::InvalidCharacterData(_) => {
            make_cant_convert_error("A Character Data node contains an invalid data. Currently, only `]]>` is not allowed.", span)
        }
        roxmltree::Error::UnknownToken(_) => {
            make_cant_convert_error("Unknown token in XML.", span)
        }
        roxmltree::Error::UnexpectedEndOfStream => {
            make_cant_convert_error("Unexpected end of stream while parsing XML.", span)
        }
    }
}

fn make_cant_convert_error(help: impl Into<String>, span: Span) -> ShellError {
    ShellError::CantConvert {
        from_type: Type::String.to_string(),
        to_type: "XML".to_string(),
        span,
        help: Some(help.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indexmap::indexmap;
    use indexmap::IndexMap;

    fn string(input: impl Into<String>) -> Value {
        Value::test_string(input)
    }

    fn attributes(entries: IndexMap<&str, &str>) -> Value {
        Value::test_record(
            entries
                .into_iter()
                .map(|(k, v)| (k.into(), string(v)))
                .collect(),
        )
    }

    fn table(list: &[Value]) -> Value {
        Value::list(list.to_vec(), Span::test_data())
    }

    fn content_tag(
        tag: impl Into<String>,
        attrs: IndexMap<&str, &str>,
        content: &[Value],
    ) -> Value {
        Value::test_record(record! {
            COLUMN_TAG_NAME =>     string(tag),
            COLUMN_ATTRS_NAME =>   attributes(attrs),
            COLUMN_CONTENT_NAME => table(content),
        })
    }

    fn content_string(value: impl Into<String>) -> Value {
        Value::test_record(record! {
            COLUMN_TAG_NAME =>     Value::nothing(Span::test_data()),
            COLUMN_ATTRS_NAME =>   Value::nothing(Span::test_data()),
            COLUMN_CONTENT_NAME => string(value),
        })
    }

    fn parse(xml: &str) -> Result<Value, roxmltree::Error> {
        let info = ParsingInfo {
            span: Span::test_data(),
            keep_comments: false,
            keep_processing_instructions: false,
        };
        from_xml_string_to_value(xml.to_string(), &info)
    }

    #[test]
    fn parses_empty_element() -> Result<(), roxmltree::Error> {
        let source = "<nu></nu>";

        assert_eq!(parse(source)?, content_tag("nu", indexmap! {}, &[]));

        Ok(())
    }

    #[test]
    fn parses_element_with_text() -> Result<(), roxmltree::Error> {
        let source = "<nu>La era de los tres caballeros</nu>";

        assert_eq!(
            parse(source)?,
            content_tag(
                "nu",
                indexmap! {},
                &[content_string("La era de los tres caballeros")]
            )
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_elements() -> Result<(), roxmltree::Error> {
        let source = "\
<nu>
    <dev>Andrés</dev>
    <dev>JT</dev>
    <dev>Yehuda</dev>
</nu>";

        assert_eq!(
            parse(source)?,
            content_tag(
                "nu",
                indexmap! {},
                &[
                    content_tag("dev", indexmap! {}, &[content_string("Andrés")]),
                    content_tag("dev", indexmap! {}, &[content_string("JT")]),
                    content_tag("dev", indexmap! {}, &[content_string("Yehuda")])
                ]
            )
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_attribute() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\">
</nu>";

        assert_eq!(
            parse(source)?,
            content_tag("nu", indexmap! {"version" => "2.0"}, &[])
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_attribute_and_element() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\">
    <version>2.0</version>
</nu>";

        assert_eq!(
            parse(source)?,
            content_tag(
                "nu",
                indexmap! {"version" => "2.0"},
                &[content_tag(
                    "version",
                    indexmap! {},
                    &[content_string("2.0")]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn parses_element_with_multiple_attributes() -> Result<(), roxmltree::Error> {
        let source = "\
<nu version=\"2.0\" age=\"25\">
</nu>";

        assert_eq!(
            parse(source)?,
            content_tag("nu", indexmap! {"version" => "2.0", "age" => "25"}, &[])
        );

        Ok(())
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromXml {})
    }
}
