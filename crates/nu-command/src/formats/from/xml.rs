use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::IndexMap;
use nu_engine::command_prelude::*;

use roxmltree::{NodeType, ParsingOptions, TextPos};

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
                "allow-dtd",
                "allow parsing documents with DTDs (may result in exponential entity expansion)",
                None,
            )
            .switch(
                "keep-pi",
                "add processing instruction nodes to result",
                None,
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .xml and create record."
    }

    fn extra_description(&self) -> &str {
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
        let allow_dtd = call.has_flag(engine_state, stack, "allow-dtd")?;
        let info = ParsingInfo {
            span: head,
            keep_comments,
            keep_processing_instructions,
            allow_dtd,
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
    allow_dtd: bool,
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

fn from_xml_string_to_value(s: &str, info: &ParsingInfo) -> Result<Value, roxmltree::Error> {
    let options = ParsingOptions {
        allow_dtd: info.allow_dtd,
        ..Default::default()
    };

    let parsed = roxmltree::Document::parse_with_options(s, options)?;
    Ok(from_document_to_value(&parsed, info))
}

fn from_xml(input: PipelineData, info: &ParsingInfo) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(info.span)?;

    match from_xml_string_to_value(&concat_string, info) {
        Ok(x) => {
            Ok(x.into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
        }
        Err(err) => Err(process_xml_parse_error(concat_string, err, span)),
    }
}

fn process_xml_parse_error(source: String, err: roxmltree::Error, span: Span) -> ShellError {
    match err {
        roxmltree::Error::InvalidXmlPrefixUri(pos) => make_xml_error_spanned(
            "The `xmlns:xml` attribute must have an <http://www.w3.org/XML/1998/namespace> URI.",
            source,
            pos,
        ),
        roxmltree::Error::UnexpectedXmlUri(pos) => make_xml_error_spanned(
            "Only the xmlns:xml attribute can have the http://www.w3.org/XML/1998/namespace  URI.",
            source,
            pos,
        ),
        roxmltree::Error::UnexpectedXmlnsUri(pos) => make_xml_error_spanned(
            "The http://www.w3.org/2000/xmlns/  URI must not be declared.",
            source,
            pos,
        ),
        roxmltree::Error::InvalidElementNamePrefix(pos) => {
            make_xml_error_spanned("xmlns can't be used as an element prefix.", source, pos)
        }
        roxmltree::Error::DuplicatedNamespace(namespace, pos) => make_xml_error_spanned(
            format!("Namespace {namespace} was already defined on this element."),
            source,
            pos,
        ),
        roxmltree::Error::UnknownNamespace(prefix, pos) => {
            make_xml_error_spanned(format!("Unknown prefix {prefix}"), source, pos)
        }
        roxmltree::Error::UnexpectedCloseTag(expected, actual, pos) => make_xml_error_spanned(
            format!("Unexpected close tag {actual}, expected {expected}"),
            source,
            pos,
        ),
        roxmltree::Error::UnexpectedEntityCloseTag(pos) => {
            make_xml_error_spanned("Entity value starts with a close tag.", source, pos)
        }
        roxmltree::Error::UnknownEntityReference(entity, pos) => make_xml_error_spanned(
            format!("Reference to unknown entity {entity} (was not defined in the DTD)"),
            source,
            pos,
        ),
        roxmltree::Error::MalformedEntityReference(pos) => {
            make_xml_error_spanned("Malformed entity reference.", source, pos)
        }
        roxmltree::Error::EntityReferenceLoop(pos) => {
            make_xml_error_spanned("Possible entity reference loop.", source, pos)
        }
        roxmltree::Error::InvalidAttributeValue(pos) => {
            make_xml_error_spanned("Attribute value cannot have a < character.", source, pos)
        }
        roxmltree::Error::DuplicatedAttribute(attribute, pos) => make_xml_error_spanned(
            format!("Element has a duplicated attribute: {attribute}"),
            source,
            pos,
        ),
        roxmltree::Error::NoRootNode => {
            make_xml_error("The XML document must have at least one element.", span)
        }
        roxmltree::Error::UnclosedRootNode => {
            make_xml_error("The root node was opened but never closed.", span)
        }
        roxmltree::Error::DtdDetected => make_xml_error(
            "XML document with DTD detected.\nDTDs are disabled by default to prevent denial-of-service attacks (use --allow-dtd to parse anyway)",
            span,
        ),
        roxmltree::Error::NodesLimitReached => make_xml_error("Node limit was reached.", span),
        roxmltree::Error::AttributesLimitReached => make_xml_error("Attribute limit reached", span),
        roxmltree::Error::NamespacesLimitReached => make_xml_error("Namespace limit reached", span),
        roxmltree::Error::UnexpectedDeclaration(pos) => make_xml_error_spanned(
            "An XML document can have only one XML declaration and it must be at the start of the document.",
            source,
            pos,
        ),
        roxmltree::Error::InvalidName(pos) => make_xml_error_spanned("Invalid name.", source, pos),
        roxmltree::Error::NonXmlChar(_, pos) => make_xml_error_spanned(
            "Non-XML character found. Valid characters are: <https://www.w3.org/TR/xml/#char32>",
            source,
            pos,
        ),
        roxmltree::Error::InvalidChar(expected, actual, pos) => make_xml_error_spanned(
            format!(
                "Unexpected character {}, expected {}",
                actual as char, expected as char
            ),
            source,
            pos,
        ),
        roxmltree::Error::InvalidChar2(expected, actual, pos) => make_xml_error_spanned(
            format!(
                "Unexpected character {}, expected {}",
                actual as char, expected
            ),
            source,
            pos,
        ),
        roxmltree::Error::InvalidString(_, pos) => {
            make_xml_error_spanned("Invalid/unexpected string in XML.", source, pos)
        }
        roxmltree::Error::InvalidExternalID(pos) => {
            make_xml_error_spanned("Invalid ExternalID in the DTD.", source, pos)
        }
        roxmltree::Error::InvalidComment(pos) => make_xml_error_spanned(
            "A comment cannot contain `--` or end with `-`.",
            source,
            pos,
        ),
        roxmltree::Error::InvalidCharacterData(pos) => make_xml_error_spanned(
            "Character Data node contains an invalid data. Currently, only `]]>` is not allowed.",
            source,
            pos,
        ),
        roxmltree::Error::UnknownToken(pos) => {
            make_xml_error_spanned("Unknown token in XML.", source, pos)
        }
        roxmltree::Error::UnexpectedEndOfStream => {
            make_xml_error("Unexpected end of stream while parsing XML.", span)
        }
    }
}

fn make_xml_error(msg: impl Into<String>, span: Span) -> ShellError {
    ShellError::GenericError {
        error: "Failed to parse XML".into(),
        msg: msg.into(),
        help: None,
        span: Some(span),
        inner: vec![],
    }
}

fn make_xml_error_spanned(msg: impl Into<String>, src: String, pos: TextPos) -> ShellError {
    let span = Span::from_row_column(pos.row as usize, pos.col as usize, &src);
    ShellError::OutsideSpannedLabeledError {
        src,
        error: "Failed to parse XML".into(),
        msg: msg.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use crate::Metadata;
    use crate::MetadataSet;
    use crate::Reject;

    use super::*;

    use indexmap::IndexMap;
    use indexmap::indexmap;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

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
            allow_dtd: false,
        };
        from_xml_string_to_value(xml, &info)
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

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(FromXml {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(MetadataSet {}));
            working_set.add_decl(Box::new(Reject {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = r#"'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | metadata set --content-type 'application/xml' --datasource-ls | from xml | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("source" => Value::test_string("ls"))),
            result.expect("There should be a result")
        )
    }
}
