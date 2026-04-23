use mq_markdown::{AttrValue, Markdown, Node};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromMd;

impl Command for FromMd {
    fn name(&self) -> &str {
        "from md"
    }

    fn description(&self) -> &str {
        "Convert markdown text into human-friendly structured rows. Use --verbose for the full AST."
    }

    fn signature(&self) -> Signature {
        Signature::build("from md")
            .input_output_types(vec![(Type::String, Type::table())])
            .switch(
                "verbose",
                "Return the full AST with type, position, attrs, and children fields.",
                Some('v'),
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "'# Title' | from md | get 0.element",
                description: "Reduced mode promotes child rows; heading text is represented as a text element.",
                result: Some(Value::test_string("text")),
            },
            Example {
                example: "'# Title' | from md | get 0.content",
                description: "Get the text content of the first element.",
                result: Some(Value::test_string("Title")),
            },
            Example {
                example: "'---
title: Demo
---
# A' | from md | get 0.element",
                description: "Parse markdown frontmatter as a dedicated yaml element.",
                result: Some(Value::test_string("yaml")),
            },
            Example {
                example: "'# Title' | from md --verbose | get 0.type",
                description: "Use --verbose to get the full AST; the first node type is h1.",
                result: Some(Value::test_string("h1")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        from_md(input, call.head, verbose)
    }
}

fn from_md(input: PipelineData, head: Span, verbose: bool) -> Result<PipelineData, ShellError> {
    let (string_input, span, metadata) = input.collect_string_strict(head)?;

    let markdown =
        Markdown::from_markdown_str(&string_input).map_err(|err| ShellError::CantConvert {
            to_type: "structured markdown data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        })?;

    let value = if verbose {
        markdown_to_ast_value(&markdown, span)
    } else {
        markdown_to_reduced_value(&markdown, span)
    };

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

/// Builds reduced rows by promoting each top-level node's immediate children to row level.
///
/// If a top-level node has no children, the node itself is emitted as a row.
/// Parent attributes are inherited by promoted child rows when child attributes are absent.
fn markdown_to_reduced_value(markdown: &Markdown, span: Span) -> Value {
    let mut nodes = Vec::new();
    for node in &markdown.nodes {
        let parent_attrs = node_reduced_attrs(node, span);
        let children = node.children();

        if children.is_empty() {
            nodes.push(node_to_reduced_value(
                node,
                span,
                parent_attrs.clone(),
                None,
            ));
        } else {
            for child in children {
                let child_attrs = node_reduced_attrs(&child, span);
                nodes.push(node_to_reduced_value(
                    &child,
                    span,
                    child_attrs,
                    parent_attrs.clone(),
                ));
            }
        }
    }

    Value::list(nodes, span)
}

fn node_to_reduced_value(
    node: &Node,
    span: Span,
    own_attrs: Option<Value>,
    inherited_attrs: Option<Value>,
) -> Value {
    let mut record = Record::new();
    record.push("element", Value::string(node_type_name(node), span));
    record.push("content", Value::string(extract_text(node), span));

    if let Some(position) = node.position() {
        record.push("content_span", position_to_value(position, span));
    }

    // Merge parent and child attributes: parent forms the base, child keys take precedence
    // on collision so the most specific information wins.
    if let Some(attrs) = merge_attrs(inherited_attrs, own_attrs, span) {
        record.push("attributes", attrs);
    }

    Value::record(record, span)
}

/// Merges two optional attribute records into one, with `child` keys overriding `parent` keys.
/// Returns `None` when both inputs are absent or produce an empty result.
fn merge_attrs(parent: Option<Value>, child: Option<Value>, span: Span) -> Option<Value> {
    match (parent, child) {
        (None, child) => child,
        (parent, None) => parent,
        (
            Some(Value::Record {
                val: parent_rec, ..
            }),
            Some(Value::Record { val: child_rec, .. }),
        ) => {
            let mut merged = (*parent_rec).clone();
            for (key, val) in child_rec.iter() {
                if let Some(existing) = merged.get_mut(key) {
                    *existing = val.clone();
                } else {
                    merged.push(key.clone(), val.clone());
                }
            }
            if merged.is_empty() {
                None
            } else {
                Some(Value::record(merged, span))
            }
        }
        // Fallback: child wins if types are unexpected
        (_, child) => child,
    }
}

/// Recursively extracts the plain-text content of a node by joining all leaf values.
fn extract_text(node: &Node) -> String {
    let children = node.children();
    if children.is_empty() {
        if let Some(AttrValue::String(s)) = node.attr("value") {
            s
        } else if node.is_text() {
            node.value().to_string()
        } else {
            String::new()
        }
    } else {
        children
            .iter()
            .map(extract_text)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Returns a reduced attributes record containing only meaningful metadata fields,
/// or `None` when no such fields are present on the node.
fn node_reduced_attrs(node: &Node, span: Span) -> Option<Value> {
    const REDUCED_ATTRIBUTE_KEYS: &[&str] = &[
        "depth", "level", "ordered", "checked", "lang", "url", "title", "alt", "align",
    ];

    let mut attrs = Record::new();
    for key in REDUCED_ATTRIBUTE_KEYS {
        if let Some(value) = node.attr(key) {
            attrs.push(*key, attr_value_to_nu_value(value, span));
        }
    }

    if attrs.is_empty() {
        None
    } else {
        Some(Value::record(attrs, span))
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
