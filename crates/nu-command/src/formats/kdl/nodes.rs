//! Nodes-mode conversion: KDL document ↔ list of node-row records.

use super::identifier_for;
use super::types::{NonRoundtrip, TypeMode, apply_type_annotation, entry_from_nu_value};
use kdl::{KdlDocument, KdlNode};
use nu_engine::command_prelude::*;
use std::collections::HashSet;

/// Convert a parsed KDL document into a list of node-row values.
pub(crate) fn nodes_document_to_value(
    document: &KdlDocument,
    span: Span,
    ignore_types: bool,
) -> Result<Value, ShellError> {
    let rows = convert_nodes(document.nodes(), span, ignore_types)?;
    Ok(Value::list(rows, span))
}

fn convert_nodes(
    nodes: &[KdlNode],
    span: Span,
    ignore_types: bool,
) -> Result<Vec<Value>, ShellError> {
    nodes
        .iter()
        .map(|node| convert_node(node, span, ignore_types))
        .collect()
}

fn convert_node(node: &KdlNode, span: Span, ignore_types: bool) -> Result<Value, ShellError> {
    let mut args = Vec::new();
    let mut props = Record::new();
    let mut used_keys: HashSet<String> = HashSet::new();

    for entry in node.entries() {
        let ty = entry.ty().map(|t| t.value());
        let typed = apply_type_annotation(
            entry.value(),
            ty,
            span,
            ignore_types,
            TypeMode::PreserveUnknown,
        )?;
        let value = typed.into_value(span);

        if let Some(name) = entry.name() {
            let key = unique_prop_key(name.value(), &mut used_keys);
            props.insert(key, value);
        } else {
            args.push(value);
        }
    }

    let children = if let Some(children_doc) = node.children() {
        convert_nodes(children_doc.nodes(), span, ignore_types)?
    } else {
        Vec::new()
    };

    let mut row = record! {
        "name" => Value::string(node.name().value(), span),
        "args" => Value::list(args, span),
        "props" => props.into_value(span),
        "children" => Value::list(children, span),
    };

    if !ignore_types && let Some(ty) = node.ty() {
        row.insert("type", Value::string(ty.value(), span));
    }

    Ok(row.into_value(span))
}

/// Encode duplicate property names as `key`, `key@2`, `key@3`, …
fn unique_prop_key(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_owned()) {
        return base.to_owned();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}@{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n = n.saturating_add(1);
        if n == u32::MAX {
            // Extremely unlikely; fall back to a longer suffix.
            let candidate = format!("{base}@dup-{n}");
            used.insert(candidate.clone());
            return candidate;
        }
    }
}

/// Convert Nu node-row value(s) into a KDL document.
pub(crate) fn node_rows_to_kdl_document(
    value: &Value,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    let rows = match value {
        Value::List { vals, .. } => vals.as_slice(),
        Value::Record { .. } if value_is_node_row(value) => std::slice::from_ref(value),
        other => {
            return Err(ShellError::UnsupportedInput {
                msg: "nodes format expects a list of node rows (or a single node row record) with name/args/props/children".into(),
                input: format!("got {}", other.get_type()),
                msg_span: call_span,
                input_span: other.span(),
            });
        }
    };

    if !rows.iter().all(value_is_node_row) {
        return Err(ShellError::UnsupportedInput {
            msg: "nodes format expects each row to be a record with name (string), args (list), props (record), children (list); optional type (string)".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: value.span(),
        });
    }

    let mut document = KdlDocument::new();
    for row in rows {
        let Value::Record { val, .. } = row else {
            unreachable!("checked by value_is_node_row");
        };
        document
            .nodes_mut()
            .push(node_row_to_kdl_node(val, non_roundtrip, call_span)?);
    }
    Ok(document)
}

pub(crate) fn value_is_node_row(value: &Value) -> bool {
    let Value::Record { val, .. } = value else {
        return false;
    };
    record_is_node_row(val)
}

fn record_is_node_row(record: &Record) -> bool {
    matches!(record.get("name"), Some(Value::String { .. }))
        && record
            .get("args")
            .is_some_and(|value| value.as_list().is_ok())
        && record
            .get("props")
            .is_some_and(|value| value.as_record().is_ok())
        && record
            .get("children")
            .is_some_and(|value| value.as_list().is_ok())
        && record
            .get("type")
            .map(|value| matches!(value, Value::String { .. } | Value::Nothing { .. }))
            .unwrap_or(true)
        // Allow only known fields so random records aren't treated as node rows.
        && record.columns().all(|col| {
            matches!(
                col.as_str(),
                "name" | "args" | "props" | "children" | "type"
            )
        })
}

fn node_row_to_kdl_node(
    row: &Record,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlNode, ShellError> {
    let name = row
        .get("name")
        .and_then(|v| v.as_str().ok())
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "node row field 'name' must be a string".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: call_span,
        })?;

    let args = row
        .get("args")
        .and_then(|v| v.as_list().ok())
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "node row field 'args' must be a list".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: call_span,
        })?;

    let props = row
        .get("props")
        .and_then(|v| v.as_record().ok())
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "node row field 'props' must be a record".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: call_span,
        })?;

    let children = row
        .get("children")
        .and_then(|v| v.as_list().ok())
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "node row field 'children' must be a list".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: call_span,
        })?;

    let mut node = KdlNode::new(identifier_for(name));

    if let Some(ty_val) = row.get("type")
        && let Ok(ty) = ty_val.as_str()
    {
        node.set_ty(identifier_for(ty));
    }

    for arg in args {
        node.push(entry_from_nu_value(arg, None, non_roundtrip, call_span)?);
    }

    for (key, value) in props.iter() {
        node.push(entry_from_nu_value(
            value,
            Some(key),
            non_roundtrip,
            call_span,
        )?);
    }

    if !children.is_empty() {
        let child_doc = node_rows_to_kdl_document(
            &Value::list(children.to_vec(), call_span),
            non_roundtrip,
            call_span,
        )?;
        node.ensure_children()
            .nodes_mut()
            .extend(child_doc.nodes().iter().cloned());
    }

    Ok(node)
}
