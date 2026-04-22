use mq_markdown::{AttrValue, Markdown, Node};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromMd;

impl Command for FromMd {
    fn name(&self) -> &str {
        "from md"
    }

    fn description(&self) -> &str {
        "Convert markdown text into structured data."
    }

    fn signature(&self) -> Signature {
        Signature::build("from md")
            .input_output_types(vec![(Type::String, Type::table())])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "'# Title' | from md | select type position attrs",
                description: "Parse markdown and return key node fields.",
                result: Some(Value::test_list(vec![heading_title_overview_node()])),
            },
            Example {
                example: "'---
title: Demo
---
# A' | from md | get 0.type",
                description: "Parse markdown frontmatter as a dedicated node.",
                result: Some(Value::test_string("yaml")),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        from_md(input, call.head)
    }
}

fn title_position(line_start: i64, column_start: i64, line_end: i64, column_end: i64) -> Value {
    Value::test_record(record! {
        "start" => Value::test_record(record! {
            "line" => Value::test_int(line_start),
            "column" => Value::test_int(column_start),
        }),
        "end" => Value::test_record(record! {
            "line" => Value::test_int(line_end),
            "column" => Value::test_int(column_end),
        }),
    })
}

fn heading_title_overview_node() -> Value {
    Value::test_record(record! {
        "type" => Value::test_string("h1"),
        "position" => title_position(1, 1, 1, 8),
        "attrs" => Value::test_record(record! {
            "depth" => Value::test_int(1),
            "level" => Value::test_int(1),
        }),
    })
}

fn from_md(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let (string_input, span, metadata) = input.collect_string_strict(head)?;

    let markdown =
        Markdown::from_markdown_str(&string_input).map_err(|err| ShellError::CantConvert {
            to_type: "structured markdown data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        })?;

    let value = markdown_to_ast_value(&markdown, span);

    Ok(value.into_pipeline_data_with_metadata(metadata.map(|md| md.with_content_type(None))))
}

fn markdown_to_ast_value(markdown: &Markdown, span: Span) -> Value {
    let nodes = markdown
        .nodes
        .iter()
        .map(|node| node_to_ast_value(node, span))
        .collect();

    Value::list(nodes, span)
}

fn node_to_ast_value(node: &Node, span: Span) -> Value {
    let children = node.children();

    let mut record = Record::new();
    record.push("type", Value::string(node_type_name(node), span));

    if let Some(position) = node.position() {
        record.push("position", position_to_value(position, span));
    } else {
        record.push("position", Value::nothing(span));
    }

    let attrs = node_attrs_to_value(node, children.is_empty(), span);
    record.push("attrs", attrs);

    let children = children
        .iter()
        .map(|child| node_to_ast_value(child, span))
        .collect();
    record.push("children", Value::list(children, span));

    Value::record(record, span)
}

fn node_type_name(node: &Node) -> String {
    if node.is_empty() {
        return "empty".to_string();
    }

    if node.is_fragment() {
        return "fragment".to_string();
    }

    node.name().to_string()
}

fn position_to_value(position: mq_markdown::Position, span: Span) -> Value {
    Value::record(
        record! {
            "start" => Value::record(
                record! {
                    "line" => Value::int(position.start.line as i64, span),
                    "column" => Value::int(position.start.column as i64, span),
                },
                span,
            ),
            "end" => Value::record(
                record! {
                    "line" => Value::int(position.end.line as i64, span),
                    "column" => Value::int(position.end.column as i64, span),
                },
                span,
            ),
        },
        span,
    )
}

fn node_attrs_to_value(node: &Node, is_leaf: bool, span: Span) -> Value {
    const ATTRIBUTE_KEYS: &[&str] = &[
        "depth", "level", "index", "ordered", "checked", "lang", "meta", "fence", "url", "title",
        "alt", "ident", "label", "row", "column", "align", "name",
    ];

    let mut attrs = Record::new();

    // Emit `value` only for leaves to avoid parent/child text duplication.
    if is_leaf {
        if let Some(value) = node.attr("value") {
            attrs.push("value", attr_value_to_nu_value(value, span));
        } else if node.is_text() {
            // Some text-like nodes can carry content without exposing a `value` attribute.
            attrs.push("value", Value::string(node.value(), span));
        }
    }

    for key in ATTRIBUTE_KEYS {
        if let Some(value) = node.attr(key) {
            attrs.push(*key, attr_value_to_nu_value(value, span));
        }
    }

    Value::record(attrs, span)
}

fn attr_value_to_nu_value(value: AttrValue, span: Span) -> Value {
    match value {
        AttrValue::String(value) => Value::string(value, span),
        AttrValue::Integer(value) => Value::int(value, span),
        AttrValue::Number(value) => Value::float(value, span),
        AttrValue::Boolean(value) => Value::bool(value, span),
        AttrValue::Null => Value::nothing(span),
        AttrValue::Array(value) => Value::list(
            value
                .iter()
                .map(|node| node_to_ast_value(node, span))
                .collect(),
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::FromMd;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromMd)
    }
}
