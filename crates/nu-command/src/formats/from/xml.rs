use crate::formats::nu_xml_format::{COLUMN_ATTRS_NAME, COLUMN_CONTENT_NAME, COLUMN_TAG_NAME};
use indexmap::IndexMap;
use nu_engine::command_prelude::*;
use nu_protocol::{
    DEFAULT_ERROR_CONTEXT, Signals, shell_error::generic::GenericError, truncated_source_window,
};
use std::collections::HashSet;

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
            .switch("keep-comments", "Add comment nodes to result.", None)
            .switch(
                "allow-dtd",
                "Allow parsing documents with DTDs (may result in exponential entity expansion).",
                None,
            )
            .switch(
                "keep-pi",
                "Add processing instruction nodes to result.",
                None,
            )
            .switch(
                "namespaces",
                "Keep namespace prefixes in tag and attribute names and add `xmlns` declarations to `attributes`.",
                None,
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse text as .xml and create record."
    }

    fn extra_description(&self) -> &str {
        r#"Every XML entry is represented via a record with tag, attributes and content fields.
To represent different types of entries different values are written to this fields:
1. Tag entry: `{tag: <tag name> attributes: {<attr name>: "<string value>" ...} content: [<entries>]}`
2. Comment entry: `{tag: '!' attributes: null content: "<comment string>"}`
3. Processing instruction (PI): `{tag: '?<pi name>' attributes: null content: "<pi content string>"}`
4. Text: `{tag: null attributes: null content: "<text>"}`.

By default names are the local names, without prefix, and namespace declarations are not shown.
With `--namespaces`, names keep the prefix as it is written in the source (`dc:title`), and each
`xmlns` or `xmlns:<prefix>` key is added to `attributes` on the element that declares it, before its
other attributes. The result can be passed to `to xml`. A sub-record keeps its prefixes but not the
`xmlns` keys of its ancestors, so add those to its `attributes` before passing it to `to xml`.

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
        let namespaces = call.has_flag(engine_state, stack, "namespaces")?;
        let info = ParsingInfo {
            span: head,
            keep_comments,
            keep_processing_instructions,
            allow_dtd,
            namespaces,
        };
        from_xml(input, &info, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"'<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml"#,
                description: "Converts xml formatted string to record.",
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
            },
            Example {
                example: r#"'<root xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Note</dc:title></root>' | from xml --namespaces"#,
                description: "Keeps namespace prefixes in names and declares them in attributes with --namespaces.",
                result: Some(Value::test_record(record! {
                    COLUMN_TAG_NAME =>     Value::test_string("root"),
                    COLUMN_ATTRS_NAME =>   Value::test_record(record! {
                        "xmlns:dc" => Value::test_string("http://purl.org/dc/elements/1.1/"),
                    }),
                    COLUMN_CONTENT_NAME => Value::test_list(vec![
                    Value::test_record(record! {
                        COLUMN_TAG_NAME =>     Value::test_string("dc:title"),
                        COLUMN_ATTRS_NAME =>   Value::test_record(Record::new()),
                        COLUMN_CONTENT_NAME => Value::test_list(vec![
                        Value::test_record(record! {
                            COLUMN_TAG_NAME =>     Value::test_nothing(),
                            COLUMN_ATTRS_NAME =>   Value::test_nothing(),
                            COLUMN_CONTENT_NAME => Value::test_string("Note"),
                            })],
                        ),
                        })],
                    ),
                })),
            },
        ]
    }
}

struct ParsingInfo {
    span: Span,
    keep_comments: bool,
    keep_processing_instructions: bool,
    allow_dtd: bool,
    /// `--namespaces`: keep prefixes in names and emit `xmlns` declarations.
    namespaces: bool,
}

/// Element name exactly as it is written in the source (`dc:creator`), read
/// from the start tag. The prefix is what the document uses, not one looked up
/// from the namespace URI. The local name is used only if the source text is
/// not available at the node's range.
fn element_name(node: &roxmltree::Node) -> String {
    let text = node.document().input_text();
    text.get(node.range().start + 1..)
        .map(|rest| {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .filter(|qname| !qname.is_empty())
        .map(String::from)
        .unwrap_or_else(|| String::from(node.tag_name().name().trim()))
}

/// Attribute name exactly as it is written in the source (`xml:space`, `p:k`).
/// The local name is used only if the source text is not available.
fn attribute_name(node: &roxmltree::Node, attribute: &roxmltree::Attribute) -> String {
    node.document()
        .input_text()
        .get(attribute.range_qname())
        .map(String::from)
        .unwrap_or_else(|| String::from(attribute.name()))
}

/// Namespace declarations (`xmlns`, `xmlns:p`) that `node` itself declares.
/// Declarations already in scope on the parent are skipped, so they are only
/// emitted on the element that introduces them.
fn namespace_declarations(node: &roxmltree::Node) -> Vec<(String, String)> {
    /// `(key, uri)` output pair for one in-scope binding.
    fn declaration(ns: &roxmltree::Namespace) -> (String, String) {
        let key = match ns.name() {
            Some(prefix) => format!("xmlns:{prefix}"),
            None => String::from("xmlns"),
        };
        (key, String::from(ns.uri()))
    }

    let Some(parent) = node.parent_element() else {
        // The root element emits every declaration it has.
        return node.namespaces().map(|ns| declaration(ns)).collect();
    };

    // Fast path, the common case: the in-scope bindings are exactly the
    // parent's. The lazy comparison stops at the first difference and allocates
    // nothing, so there is nothing new to emit.
    if node
        .namespaces()
        .map(|ns| (ns.name(), ns.uri()))
        .eq(parent.namespaces().map(|ns| (ns.name(), ns.uri())))
    {
        return Vec::new();
    }

    // Slow path: some binding is new or changed. Build one set of the parent's
    // bindings and emit only the in-scope bindings it does not contain.
    let inherited: HashSet<(Option<&str>, &str)> = parent
        .namespaces()
        .map(|ns| (ns.name(), ns.uri()))
        .collect();
    node.namespaces()
        .filter(|ns| !inherited.contains(&(ns.name(), ns.uri())))
        .map(|ns| declaration(ns))
        .collect()
}

/// Attributes of `node` as a record. With `--namespaces` the `xmlns`
/// declarations come first and attribute names keep their prefix. Without it
/// the output is the plain local names of the base command, and no namespace
/// information is read.
fn from_attributes_to_value(node: &roxmltree::Node, info: &ParsingInfo) -> Value {
    let mut collected = IndexMap::new();
    if info.namespaces {
        for (key, uri) in namespace_declarations(node) {
            collected.insert(key, Value::string(uri, info.span));
        }
    }
    for a in node.attributes() {
        let name = if info.namespaces {
            attribute_name(node, &a)
        } else {
            String::from(a.name())
        };
        collected.insert(name, Value::string(a.value(), info.span));
    }
    Value::record(collected.into_iter().collect(), info.span)
}

fn element_to_value(n: &roxmltree::Node, info: &ParsingInfo) -> Value {
    let span = info.span;
    let mut node = IndexMap::new();

    let name = n.tag_name();
    let tag = if info.namespaces {
        element_name(n)
    } else {
        String::from(name.name().trim())
    };
    let tag = Value::string(tag, span);

    let content: Vec<Value> = n
        .children()
        .filter_map(|node| from_node_to_value(&node, info))
        .collect();
    let content = Value::list(content, span);

    let attributes = from_attributes_to_value(n, info);

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

fn from_xml(
    input: PipelineData,
    info: &ParsingInfo,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(info.span)?;

    match from_xml_string_to_value(&concat_string, info) {
        Ok(x) => {
            Ok(x.into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
        }
        Err(err) => Err(process_xml_parse_error(concat_string, err, span, signals)),
    }
}

fn process_xml_parse_error(
    source: impl AsRef<str>,
    err: roxmltree::Error,
    span: Span,
    signals: &Signals,
) -> ShellError {
    let source = source.as_ref();
    match err {
        roxmltree::Error::InvalidXmlPrefixUri(pos) => make_xml_err(
            source,
            span,
            signals,
            "The `xmlns:xml` attribute must have an <http://www.w3.org/XML/1998/namespace> URI.",
            pos,
        ),
        roxmltree::Error::UnexpectedXmlUri(pos) => make_xml_err(
            source,
            span,
            signals,
            "Only the xmlns:xml attribute can have the http://www.w3.org/XML/1998/namespace  URI.",
            pos,
        ),
        roxmltree::Error::UnexpectedXmlnsUri(pos) => make_xml_err(
            source,
            span,
            signals,
            "The http://www.w3.org/2000/xmlns/  URI must not be declared.",
            pos,
        ),
        roxmltree::Error::InvalidElementNamePrefix(pos) => make_xml_err(
            source,
            span,
            signals,
            "xmlns can't be used as an element prefix.",
            pos,
        ),
        roxmltree::Error::DuplicatedNamespace(namespace, pos) => make_xml_err(
            source,
            span,
            signals,
            format!("Namespace {namespace} was already defined on this element."),
            pos,
        ),
        roxmltree::Error::UnknownNamespace(prefix, pos) => make_xml_err(
            source,
            span,
            signals,
            format!("Unknown prefix {prefix}"),
            pos,
        ),
        roxmltree::Error::UnexpectedCloseTag(expected, actual, pos) => make_xml_err(
            source,
            span,
            signals,
            format!("Unexpected close tag {actual}, expected {expected}"),
            pos,
        ),
        roxmltree::Error::UnexpectedEntityCloseTag(pos) => make_xml_err(
            source,
            span,
            signals,
            "Entity value starts with a close tag.",
            pos,
        ),
        roxmltree::Error::UnknownEntityReference(entity, pos) => make_xml_err(
            source,
            span,
            signals,
            format!("Reference to unknown entity {entity} (was not defined in the DTD)"),
            pos,
        ),
        roxmltree::Error::MalformedEntityReference(pos) => {
            make_xml_err(source, span, signals, "Malformed entity reference.", pos)
        }
        roxmltree::Error::EntityReferenceLoop(pos) => make_xml_err(
            source,
            span,
            signals,
            "Possible entity reference loop.",
            pos,
        ),
        roxmltree::Error::InvalidAttributeValue(pos) => make_xml_err(
            source,
            span,
            signals,
            "Attribute value cannot have a < character.",
            pos,
        ),
        roxmltree::Error::DuplicatedAttribute(attribute, pos) => make_xml_err(
            source,
            span,
            signals,
            format!("Element has a duplicated attribute: {attribute}"),
            pos,
        ),
        roxmltree::Error::NoRootNode => {
            make_xml_error("The XML document must have at least one element.", span)
        }
        roxmltree::Error::UnclosedRootNode => {
            make_xml_error("The root node was opened but never closed.", span)
        }
        roxmltree::Error::DtdDetected => make_xml_error(
            "XML document with DTD detected.\nDTDs are disabled by default to prevent denial-of-service attacks (use `from xml --allow-dtd` to bypass this functionality)",
            span,
        ),
        roxmltree::Error::NodesLimitReached => make_xml_error("Node limit was reached.", span),
        roxmltree::Error::AttributesLimitReached => make_xml_error("Attribute limit reached", span),
        roxmltree::Error::NamespacesLimitReached => make_xml_error("Namespace limit reached", span),
        roxmltree::Error::UnexpectedDeclaration(pos) => make_xml_err(
            source,
            span,
            signals,
            "An XML document can have only one XML declaration and it must be at the start of the document.",
            pos,
        ),
        roxmltree::Error::InvalidName(pos) => {
            make_xml_err(source, span, signals, "Invalid name.", pos)
        }
        roxmltree::Error::NonXmlChar(_, pos) => make_xml_err(
            source,
            span,
            signals,
            "Non-XML character found. Valid characters are: <https://www.w3.org/TR/xml/#char32>",
            pos,
        ),
        roxmltree::Error::InvalidChar(expected, actual, pos) => make_xml_err(
            source,
            span,
            signals,
            format!(
                "Unexpected character {}, expected {}",
                actual as char, expected as char
            ),
            pos,
        ),
        roxmltree::Error::InvalidChar2(expected, actual, pos) => make_xml_err(
            source,
            span,
            signals,
            format!(
                "Unexpected character {}, expected {}",
                actual as char, expected
            ),
            pos,
        ),
        roxmltree::Error::InvalidString(_, pos) => make_xml_err(
            source,
            span,
            signals,
            "Invalid/unexpected string in XML.",
            pos,
        ),
        roxmltree::Error::InvalidExternalID(pos) => {
            make_xml_err(source, span, signals, "Invalid ExternalID in the DTD.", pos)
        }
        roxmltree::Error::EntityResolver(pos, msg) => make_xml_err(
            source,
            span,
            signals,
            format!("Resolving the given entity yielded an error: {msg}."),
            pos,
        ),
        roxmltree::Error::InvalidComment(pos) => make_xml_err(
            source,
            span,
            signals,
            "A comment cannot contain `--` or end with `-`.",
            pos,
        ),
        roxmltree::Error::InvalidCharacterData(pos) => make_xml_err(
            source,
            span,
            signals,
            "Character Data node contains an invalid data. Currently, only `]]>` is not allowed.",
            pos,
        ),
        roxmltree::Error::UnknownToken(pos) => {
            make_xml_err(source, span, signals, "Unknown token in XML.", pos)
        }
        roxmltree::Error::UnexpectedEndOfStream => {
            make_xml_error("Unexpected end of stream while parsing XML.", span)
        }
    }
}

fn make_xml_err(
    source: &str,
    span: Span,
    signals: &Signals,
    msg: impl Into<String>,
    pos: TextPos,
) -> ShellError {
    match Span::try_from_row_column(pos.row as usize, pos.col as usize, source, &span, signals) {
        Ok(byte_span) => {
            let (src, label_span) =
                truncated_source_window(source, byte_span, DEFAULT_ERROR_CONTEXT);
            ShellError::OutsideSpannedLabeledError {
                src,
                error: "Failed to parse XML".into(),
                msg: msg.into(),
                span: label_span,
            }
        }
        Err(e) => e,
    }
}

fn make_xml_error(msg: impl Into<String>, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new("Failed to parse XML", msg.into(), span))
}

#[cfg(test)]
mod tests {
    use crate::Metadata;
    use crate::MetadataSet;
    use crate::Reject;
    use roxmltree::ParsingOptions;

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

    /// Parses with the default options (no `--namespaces`).
    fn parse(xml: &str) -> Result<Value, roxmltree::Error> {
        parse_with(xml, false)
    }

    /// Parses with `--namespaces`.
    fn parse_namespaces(xml: &str) -> Result<Value, roxmltree::Error> {
        parse_with(xml, true)
    }

    fn parse_with(xml: &str, namespaces: bool) -> Result<Value, roxmltree::Error> {
        let info = ParsingInfo {
            span: Span::test_data(),
            keep_comments: false,
            keep_processing_instructions: false,
            allow_dtd: false,
            namespaces,
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
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromXml)
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
</note>' | metadata set --content-type 'application/xml' --path-columns [name] | from xml | metadata | reject span | $in"#;
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(
                record!("path_columns" => Value::test_list(vec![Value::test_string("name")]))
            ),
            result.expect("There should be a result")
        )
    }

    #[test]
    fn xml_error_source_is_bounded() {
        // Build a large valid XML with an error near the end
        let mut input = String::from("<root>");
        for _ in 0..5000 {
            input.push_str("<item>value</item>");
        }
        input.push_str("<bad"); // Unclosed tag at the end (error)

        let signals = Signals::empty();
        let parse_result = roxmltree::Document::parse_with_options(
            &input,
            ParsingOptions {
                allow_dtd: true,
                ..Default::default()
            },
        );
        assert!(parse_result.is_err(), "should fail to parse");

        let err = process_xml_parse_error(
            &input,
            parse_result.unwrap_err(),
            Span::test_data(),
            &signals,
        );
        match &err {
            ShellError::OutsideSpannedLabeledError { src, .. } => {
                assert!(
                    src.len() < 20_000,
                    "error source should be bounded, got {} bytes",
                    src.len()
                );
            }
            ShellError::Generic(_) => (), // Generic errors without source are also OK
            other => panic!("expected OutsideSpannedLabeledError or Generic, got {other:?}"),
        }
    }

    #[test]
    fn xml_parse_success_not_affected() {
        let result = from_xml_string_to_value(
            r#"<?xml version="1.0"?><root><item>value</item></root>"#,
            &ParsingInfo {
                span: Span::test_data(),
                keep_comments: false,
                keep_processing_instructions: false,
                allow_dtd: false,
                namespaces: false,
            },
        );
        assert!(result.is_ok(), "valid XML should still parse");
    }

    #[test]
    fn parses_document_without_namespaces_unchanged() -> Result<(), roxmltree::Error> {
        let source = r#"<a href="x"><b>t</b></a>"#;

        assert_eq!(
            parse(source)?,
            content_tag(
                "a",
                indexmap! {"href" => "x"},
                &[content_tag("b", indexmap! {}, &[content_string("t")])]
            )
        );

        Ok(())
    }

    #[test]
    fn default_mode_uses_local_names_and_no_xmlns_keys() -> Result<(), roxmltree::Error> {
        // Without `--namespaces` the output is the base output: local names,
        // no `xmlns` keys, and the prefix is dropped from attribute names.
        let source = r#"<a xmlns="u" xmlns:p="u"><p:b p:k="1" xml:lang="en">t</p:b></a>"#;

        assert_eq!(
            parse(source)?,
            content_tag(
                "a",
                indexmap! {},
                &[content_tag(
                    "b",
                    indexmap! {"k" => "1", "lang" => "en"},
                    &[content_string("t")]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn element_and_attribute_keep_prefix_when_default_namespace_is_also_bound()
    -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns="u" xmlns:p="u"><p:b p:k="1">t</p:b></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns" => "u", "xmlns:p" => "u"},
                &[content_tag(
                    "p:b",
                    indexmap! {"p:k" => "1"},
                    &[content_string("t")]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn empty_default_namespace_is_emitted_as_undeclaration() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns="u"><b xmlns=""/></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns" => "u"},
                &[content_tag("b", indexmap! {"xmlns" => ""}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn prefix_redeclared_on_child_with_other_uri_is_emitted_on_child()
    -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:p="u1"><b xmlns:p="u2"><p:c/></b></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p" => "u1"},
                &[content_tag(
                    "b",
                    indexmap! {"xmlns:p" => "u2"},
                    &[content_tag("p:c", indexmap! {}, &[])]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn prefixed_attribute_under_other_default_namespace() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns="d" xmlns:p="u"><b p:k="1"/></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns" => "d", "xmlns:p" => "u"},
                &[content_tag("b", indexmap! {"p:k" => "1"}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn xml_lang_attribute_key_is_xml_prefixed() -> Result<(), roxmltree::Error> {
        let source = r#"<a xml:lang="en"/>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag("a", indexmap! {"xml:lang" => "en"}, &[])
        );

        Ok(())
    }

    #[test]
    fn xml_lang_attribute_key_with_explicit_xml_declaration() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:xml="http://www.w3.org/XML/1998/namespace" xml:lang="en"/>"#;

        // roxmltree does not expose an explicit `xmlns:xml` declaration as a
        // namespace, so only the attribute key is checked here.
        assert_eq!(
            parse_namespaces(source)?,
            content_tag("a", indexmap! {"xml:lang" => "en"}, &[])
        );

        Ok(())
    }

    #[test]
    fn many_declarations_are_emitted_only_on_the_root() -> Result<(), roxmltree::Error> {
        const COUNT: usize = 50;
        let decls: Vec<(String, String)> = (0..COUNT)
            .map(|i| (format!("xmlns:p{i}"), format!("urn:p{i}")))
            .collect();
        let root_attrs: IndexMap<&str, &str> = decls
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let xmlns_attrs: String = decls.iter().map(|(k, v)| format!(" {k}=\"{v}\"")).collect();
        let children: String = (0..COUNT).map(|i| format!("<p{i}:e/>")).collect();
        let source = format!("<r{xmlns_attrs}>{children}</r>");

        let expected_children: Vec<Value> = (0..COUNT)
            .map(|i| content_tag(format!("p{i}:e"), indexmap! {}, &[]))
            .collect();

        assert_eq!(
            parse_namespaces(&source)?,
            content_tag("r", root_attrs, &expected_children)
        );

        Ok(())
    }

    #[test]
    fn child_redeclaring_same_prefix_and_uri_emits_nothing() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:p="u"><b xmlns:p="u"><p:c/></b></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p" => "u"},
                &[content_tag(
                    "b",
                    indexmap! {},
                    &[content_tag("p:c", indexmap! {}, &[])]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn child_adding_one_declaration_among_inherited_ones_emits_only_that_one()
    -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:p0="u0" xmlns:p1="u1" xmlns:p2="u2" xmlns:p3="u3" xmlns:p4="u4"><c xmlns:p5="u5"><p0:e/></c></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p0" => "u0", "xmlns:p1" => "u1", "xmlns:p2" => "u2", "xmlns:p3" => "u3", "xmlns:p4" => "u4"},
                &[content_tag(
                    "c",
                    indexmap! {"xmlns:p5" => "u5"},
                    &[content_tag("p0:e", indexmap! {}, &[])]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn svg_default_and_prefix_for_same_uri_keeps_written_prefix() -> Result<(), roxmltree::Error> {
        let source = r#"<svg xmlns="s" xmlns:svg="s"><svg:rect width="1"/></svg>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "svg",
                indexmap! {"xmlns" => "s", "xmlns:svg" => "s"},
                &[content_tag("svg:rect", indexmap! {"width" => "1"}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn second_prefix_for_same_uri_keeps_written_prefix() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:p="u" xmlns:q="u"><q:b q:k="1"/></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p" => "u", "xmlns:q" => "u"},
                &[content_tag("q:b", indexmap! {"q:k" => "1"}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn prefix_bound_on_ancestor_keeps_written_name() -> Result<(), roxmltree::Error> {
        let source = r#"<a xmlns:dc="X"><b xmlns:x="X"><dc:t/></b></a>"#;

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:dc" => "X"},
                &[content_tag(
                    "b",
                    indexmap! {"xmlns:x" => "X"},
                    &[content_tag("dc:t", indexmap! {}, &[])]
                )]
            )
        );

        Ok(())
    }

    #[test]
    fn names_are_read_from_source_across_newlines_and_tabs() -> Result<(), roxmltree::Error> {
        let source = "<a\n xmlns:p=\"u\"\n><p:b\n\tp:k=\"1\"/></a>";

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p" => "u"},
                &[content_tag("p:b", indexmap! {"p:k" => "1"}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn names_are_read_from_source_after_bom_comment_and_pi() -> Result<(), roxmltree::Error> {
        let source = "\u{feff}<!-- c --><?pi x?><a xmlns:p=\"u\"><p:b p:k=\"1\"/></a>";

        assert_eq!(
            parse_namespaces(source)?,
            content_tag(
                "a",
                indexmap! {"xmlns:p" => "u"},
                &[content_tag("p:b", indexmap! {"p:k" => "1"}, &[])]
            )
        );

        Ok(())
    }

    #[test]
    fn dtd_entity_element_does_not_panic() {
        let source = "<!DOCTYPE a [<!ENTITY e \"<p:b xmlns:p='u'/>\">]><a xmlns:p=\"u\">&e;</a>";
        let info = ParsingInfo {
            span: Span::test_data(),
            keep_comments: false,
            keep_processing_instructions: false,
            allow_dtd: true,
            namespaces: true,
        };
        let value = from_xml_string_to_value(source, &info);
        // Report what the name looks like; the point is that it does not panic.
        eprintln!("dtd entity result: {value:?}");
        assert!(value.is_ok());
    }
}
