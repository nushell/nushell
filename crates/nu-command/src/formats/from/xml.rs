use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::map::IndexMap;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SpannedValue, Type,
};
use roxmltree::NodeType;

#[derive(Clone)]
pub struct FromXml;

impl Command for FromXml {
    fn name(&self) -> &str {
        "from xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from xml")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let keep_comments = call.has_flag("keep-comments");
        let keep_processing_instructions = call.has_flag("keep-pi");
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
            result: Some(SpannedValue::test_record(
                vec![COLUMN_TAG_NAME, COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME],
                vec![
                    SpannedValue::test_string("note"),
                    SpannedValue::test_record(Vec::<&str>::new(), vec![]),
                    SpannedValue::list(
                        vec![SpannedValue::test_record(
                            vec![COLUMN_TAG_NAME, COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME],
                            vec![
                                SpannedValue::test_string("remember"),
                                SpannedValue::test_record(Vec::<&str>::new(), vec![]),
                                SpannedValue::list(
                                    vec![SpannedValue::test_record(
                                        vec![
                                            COLUMN_TAG_NAME,
                                            COLUMN_ATTRS_NAME,
                                            COLUMN_CONTENT_NAME,
                                        ],
                                        vec![
                                            SpannedValue::test_nothing(),
                                            SpannedValue::test_nothing(),
                                            SpannedValue::test_string("Event"),
                                        ],
                                    )],
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Span::test_data(),
                    ),
                ],
            )),
        }]
    }
}

struct ParsingInfo {
    span: Span,
    keep_comments: bool,
    keep_processing_instructions: bool,
}

fn from_attributes_to_value(
    attributes: &[roxmltree::Attribute],
    info: &ParsingInfo,
) -> SpannedValue {
    let mut collected = IndexMap::new();
    for a in attributes {
        collected.insert(
            String::from(a.name()),
            SpannedValue::string(a.value(), info.span),
        );
    }

    let (cols, vals) = collected
        .into_iter()
        .fold((vec![], vec![]), |mut acc, (k, v)| {
            acc.0.push(k);
            acc.1.push(v);
            acc
        });

    SpannedValue::Record {
        cols,
        vals,
        span: info.span,
    }
}

fn element_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> SpannedValue {
    let span = info.span;
    let mut node = IndexMap::new();

    let tag = n.tag_name().name().trim().to_string();
    let tag = SpannedValue::string(tag, span);

    let content: Vec<SpannedValue> = n
        .children()
        .filter_map(|node| from_node_to_value(&node, info))
        .collect();
    let content = SpannedValue::list(content, span);

    let attributes = from_attributes_to_value(&n.attributes().collect::<Vec<_>>(), info);

    node.insert(String::from(COLUMN_TAG_NAME), tag);
    node.insert(String::from(COLUMN_ATTRS_NAME), attributes);
    node.insert(String::from(COLUMN_CONTENT_NAME), content);

    SpannedValue::from(Spanned { item: node, span })
}

fn text_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<SpannedValue> {
    let span = info.span;
    let text = n.text().expect("Non-text node supplied to text_to_value");
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        let mut node = IndexMap::new();
        let content = SpannedValue::string(String::from(text), span);

        node.insert(String::from(COLUMN_TAG_NAME), SpannedValue::nothing(span));
        node.insert(String::from(COLUMN_ATTRS_NAME), SpannedValue::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = SpannedValue::from(Spanned { item: node, span });

        Some(result)
    }
}

fn comment_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<SpannedValue> {
    if info.keep_comments {
        let span = info.span;
        let text = n
            .text()
            .expect("Non-comment node supplied to comment_to_value");

        let mut node = IndexMap::new();
        let content = SpannedValue::string(String::from(text), span);

        node.insert(
            String::from(COLUMN_TAG_NAME),
            SpannedValue::string("!", span),
        );
        node.insert(String::from(COLUMN_ATTRS_NAME), SpannedValue::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = SpannedValue::from(Spanned { item: node, span });

        Some(result)
    } else {
        None
    }
}

fn processing_instruction_to_value(
    n: &roxmltree::Node,
    info: &ParsingInfo,
) -> Option<SpannedValue> {
    if info.keep_processing_instructions {
        let span = info.span;
        let pi = n.pi()?;

        let mut node = IndexMap::new();
        // Add '?' before target to differentiate tags from pi targets
        let tag = format!("?{}", pi.target);
        let tag = SpannedValue::string(tag, span);
        let content = pi.value.map_or_else(
            || SpannedValue::nothing(span),
            |x| SpannedValue::string(x, span),
        );

        node.insert(String::from(COLUMN_TAG_NAME), tag);
        node.insert(String::from(COLUMN_ATTRS_NAME), SpannedValue::nothing(span));
        node.insert(String::from(COLUMN_CONTENT_NAME), content);

        let result = SpannedValue::from(Spanned { item: node, span });

        Some(result)
    } else {
        None
    }
}

fn from_node_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Option<SpannedValue> {
    match n.node_type() {
        NodeType::Element => Some(element_to_value(n, info)),
        NodeType::Text => text_to_value(n, info),
        NodeType::Comment => comment_to_value(n, info),
        NodeType::PI => processing_instruction_to_value(n, info),
        _ => None,
    }
}

fn from_document_to_value(d: &roxmltree::Document, info: &ParsingInfo) -> SpannedValue {
    element_to_value(&d.root_element(), info)
}

fn from_xml_string_to_value(
    s: String,
    info: &ParsingInfo,
) -> Result<SpannedValue, roxmltree::Error> {
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
        roxmltree::Error::ParserError(_) => make_cant_convert_error("Parser error", span),
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
    use nu_protocol::{Spanned, SpannedValue};

    fn string(input: impl Into<String>) -> SpannedValue {
        SpannedValue::test_string(input)
    }

    fn attributes(entries: IndexMap<&str, &str>) -> SpannedValue {
        SpannedValue::from(Spanned {
            item: entries
                .into_iter()
                .map(|(k, v)| (k.into(), string(v)))
                .collect::<IndexMap<String, SpannedValue>>(),
            span: Span::test_data(),
        })
    }

    fn table(list: &[SpannedValue]) -> SpannedValue {
        SpannedValue::List {
            vals: list.to_vec(),
            span: Span::test_data(),
        }
    }

    fn content_tag(
        tag: impl Into<String>,
        attrs: IndexMap<&str, &str>,
        content: &[SpannedValue],
    ) -> SpannedValue {
        SpannedValue::from(Spanned {
            item: indexmap! {
                COLUMN_TAG_NAME.into() => string(tag),
                COLUMN_ATTRS_NAME.into() => attributes(attrs),
                COLUMN_CONTENT_NAME.into() => table(content),
            },
            span: Span::test_data(),
        })
    }

    fn content_string(value: impl Into<String>) -> SpannedValue {
        SpannedValue::from(Spanned {
            item: indexmap! {
                COLUMN_TAG_NAME.into() => SpannedValue::nothing(Span::test_data()),
                COLUMN_ATTRS_NAME.into() => SpannedValue::nothing(Span::test_data()),
                COLUMN_CONTENT_NAME.into() => string(value),
            },
            span: Span::test_data(),
        })
    }

    fn parse(xml: &str) -> Result<SpannedValue, roxmltree::Error> {
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
                &vec![
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
