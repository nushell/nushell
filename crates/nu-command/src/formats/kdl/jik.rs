//! JSON-in-KDL (JiK) mapping between Nushell values and KDL documents.
//!
//! Spec: https://github.com/kdl-org/kdl/blob/main/JSON-IN-KDL.md

use super::identifier_for;
use super::types::{
    NonRoundtrip, TY_ARRAY, TY_OBJECT, TypeMode, apply_type_annotation, entry_from_nu_value,
    is_kdl_literal,
};
use kdl::{KdlDocument, KdlNode};
use nu_engine::command_prelude::*;

const ANON: &str = "-";

/// Parse a JiK document into a single Nushell value.
pub(crate) fn jik_document_to_value(
    document: &KdlDocument,
    span: Span,
    ignore_types: bool,
) -> Result<Value, ShellError> {
    let nodes = document.nodes();
    if nodes.is_empty() {
        return Err(ShellError::CantConvert {
            to_type: "JiK value".into(),
            from_type: "empty KDL document".into(),
            span,
            help: Some(
                "JiK expects a single top-level node; use --format nodes for documents".into(),
            ),
        });
    }
    if nodes.len() > 1 {
        return Err(ShellError::CantConvert {
            to_type: "JiK value".into(),
            from_type: "multi-node KDL document".into(),
            span,
            help: Some(
                "JiK expects a single top-level node (use --format nodes for full documents)"
                    .into(),
            ),
        });
    }
    jik_node_to_value(&nodes[0], span, ignore_types)
}

fn jik_node_to_value(node: &KdlNode, span: Span, ignore_types: bool) -> Result<Value, ShellError> {
    let node_ty = node.ty().map(|t| t.value());

    // Reject unknown node-level type annotations in strict JiK (except array/object).
    if let Some(ty) = node_ty
        && ty != TY_ARRAY
        && ty != TY_OBJECT
        && !ignore_types
        && !is_known_value_type(ty)
    {
        return Err(unknown_jik_type(ty, span));
    }

    let entries = node.entries();
    let children = node.children().map(|d| d.nodes()).unwrap_or(&[]);

    let has_props = entries.iter().any(|e| e.name().is_some());
    let has_args = entries.iter().any(|e| e.name().is_none());
    let all_children_anon = children.iter().all(|c| c.name().value() == ANON);
    let any_children = !children.is_empty();

    let is_array_annotated = node_ty == Some(TY_ARRAY);
    let is_object_annotated = node_ty == Some(TY_OBJECT);

    // Empty node
    if entries.is_empty() && !any_children {
        return match node_ty {
            Some(TY_ARRAY) => Ok(Value::list(vec![], span)),
            Some(TY_OBJECT) => Ok(Value::record(Record::new(), span)),
            _ => Err(ShellError::CantConvert {
                to_type: "JiK value".into(),
                from_type: "empty KDL node without (array) or (object)".into(),
                span,
                help: Some(
                    "empty array is `(array)-`; empty object is `(object)-`. Or use --format nodes."
                        .into(),
                ),
            }),
        };
    }

    // Literal node: single unnamed argument, no props, no children
    // Ambiguous with single-element array unless (array) is present.
    if !has_props && !any_children && entries.len() == 1 && entries[0].name().is_none() {
        if is_array_annotated {
            let item = entry_to_jik_literal(&entries[0], span, ignore_types)?;
            return Ok(Value::list(vec![item], span));
        }
        if is_object_annotated {
            return Err(invalid_jik(
                "object node cannot be a single bare argument",
                span,
            ));
        }
        return entry_to_jik_literal(&entries[0], span, ignore_types);
    }

    // Array: only unnamed args and/or `-` children
    let looks_like_array =
        !has_props && (has_args || any_children) && all_children_anon && !is_object_annotated;

    // Object: only named props and/or children (any names), no? wait - arrays can't have non-anon children
    let looks_like_object = !has_args && (has_props || any_children) && !is_array_annotated;

    // Mixed args+props is invalid JiK for both (except we allow array with only args)
    if has_args && has_props {
        return Err(invalid_jik(
            "JiK node cannot mix unnamed arguments and properties",
            span,
        ));
    }

    if is_array_annotated || (looks_like_array && !looks_like_object) {
        if has_props || !all_children_anon {
            return Err(invalid_jik(
                "JiK array nodes may only have unnamed arguments and '-' children",
                span,
            ));
        }
        return jik_array_to_value(entries, children, span, ignore_types);
    }

    if is_object_annotated || looks_like_object {
        if has_args {
            return Err(invalid_jik(
                "JiK object nodes may only have properties and children",
                span,
            ));
        }
        // Ambiguous: single child named "-" with no props could be array or object.
        // JiK says object with sole key "-" written as children needs (object).
        if !has_props
            && children.len() == 1
            && children[0].name().value() == ANON
            && !is_object_annotated
            && all_children_anon
        {
            // Treat as array of one complex element (common case) unless (object)
            return jik_array_to_value(entries, children, span, ignore_types);
        }
        return jik_object_to_value(entries, children, span, ignore_types);
    }

    // Args + non-anon children mixed: invalid
    if has_args && any_children && !all_children_anon {
        return Err(invalid_jik("JiK array children must be named '-'", span));
    }

    Err(invalid_jik(
        "node is not valid JSON-in-KDL (try --format nodes)",
        span,
    ))
}

fn jik_array_to_value(
    entries: &[kdl::KdlEntry],
    children: &[KdlNode],
    span: Span,
    ignore_types: bool,
) -> Result<Value, ShellError> {
    let mut items = Vec::new();
    for entry in entries {
        if entry.name().is_some() {
            return Err(invalid_jik("JiK array cannot have properties", span));
        }
        items.push(entry_to_jik_literal(entry, span, ignore_types)?);
    }
    for child in children {
        if child.name().value() != ANON {
            return Err(invalid_jik("JiK array children must be named '-'", span));
        }
        items.push(jik_node_to_value(child, span, ignore_types)?);
    }
    Ok(Value::list(items, span))
}

fn jik_object_to_value(
    entries: &[kdl::KdlEntry],
    children: &[KdlNode],
    span: Span,
    ignore_types: bool,
) -> Result<Value, ShellError> {
    let mut record = Record::new();
    let mut keys = std::collections::HashSet::new();

    for entry in entries {
        let Some(name) = entry.name() else {
            return Err(invalid_jik(
                "JiK object cannot have unnamed arguments",
                span,
            ));
        };
        let key = name.value().to_string();
        if !keys.insert(key.clone()) {
            return Err(invalid_jik(
                &format!("duplicate object key '{key}' in JiK node"),
                span,
            ));
        }
        record.insert(key, entry_to_jik_literal(entry, span, ignore_types)?);
    }

    for child in children {
        let key = child.name().value().to_string();
        if !keys.insert(key.clone()) {
            return Err(invalid_jik(
                &format!("duplicate object key '{key}' in JiK node"),
                span,
            ));
        }
        // Child encodes key; value is the JiK interpretation of the child node.
        record.insert(key, jik_node_to_value(child, span, ignore_types)?);
    }

    Ok(record.into_value(span))
}

fn entry_to_jik_literal(
    entry: &kdl::KdlEntry,
    span: Span,
    ignore_types: bool,
) -> Result<Value, ShellError> {
    let ty = entry.ty().map(|t| t.value());
    if let Some(t) = ty
        && (t == TY_ARRAY || t == TY_OBJECT)
    {
        return Err(invalid_jik(
            "array/object type annotations belong on nodes, not entry values",
            span,
        ));
    }
    let typed = apply_type_annotation(entry.value(), ty, span, ignore_types, TypeMode::Strict)?;
    Ok(typed.into_value(span))
}

fn is_known_value_type(ty: &str) -> bool {
    matches!(
        ty,
        super::types::TY_FILESIZE
            | super::types::TY_DURATION
            | super::types::TY_TIMESTAMP
            | super::types::TY_BINARY
            | super::types::TY_GLOB
            | super::types::TY_RANGE
            | super::types::TY_CELL_PATH
    )
}

fn unknown_jik_type(ty: &str, span: Span) -> ShellError {
    ShellError::UnsupportedInput {
        msg: format!(
            "unknown KDL type annotation '{ty}' in JiK mode; use --ignore-types or --format nodes"
        ),
        input: "value originates from here".into(),
        msg_span: span,
        input_span: span,
    }
}

fn invalid_jik(msg: &str, span: Span) -> ShellError {
    ShellError::CantConvert {
        to_type: "JiK / Nushell value".into(),
        from_type: "KDL".into(),
        span,
        help: Some(format!(
            "{msg}. Hint: use `from kdl --format nodes` for full KDL documents."
        )),
    }
}

/// Serialize a Nushell value as a JiK document (single top-level `-` node).
pub(crate) fn value_to_jik_document(
    value: &Value,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    let node = value_to_jik_node(value, ANON, non_roundtrip, call_span)?;
    let mut document = KdlDocument::new();
    document.nodes_mut().push(node);
    Ok(document)
}

fn value_to_jik_node(
    value: &Value,
    name: &str,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlNode, ShellError> {
    match value {
        Value::List { vals, .. } => list_to_jik_node(vals, name, non_roundtrip, call_span),
        Value::Record { val, .. } => record_to_jik_node(val, name, non_roundtrip, call_span),
        literal => {
            let mut node = KdlNode::new(identifier_for(name));
            node.push(entry_from_nu_value(
                literal,
                None,
                non_roundtrip,
                call_span,
            )?);
            Ok(node)
        }
    }
}

fn list_to_jik_node(
    vals: &[Value],
    name: &str,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlNode, ShellError> {
    let mut node = KdlNode::new(identifier_for(name));

    if vals.is_empty() {
        node.set_ty(identifier_for(TY_ARRAY));
        return Ok(node);
    }

    // Single-element array of a literal must be annotated (array) when using args.
    if vals.len() == 1 && is_kdl_literal(&vals[0]) {
        node.set_ty(identifier_for(TY_ARRAY));
        node.push(entry_from_nu_value(
            &vals[0],
            None,
            non_roundtrip,
            call_span,
        )?);
        return Ok(node);
    }

    // All literals → arguments on one node (`- 1 2 3`).
    if vals.iter().all(is_kdl_literal) {
        for val in vals {
            node.push(entry_from_nu_value(val, None, non_roundtrip, call_span)?);
        }
        return Ok(node);
    }

    // Any nested structures → each item as a `-` child node.
    let mut child_doc = KdlDocument::new();
    for val in vals {
        child_doc
            .nodes_mut()
            .push(value_to_jik_node(val, ANON, non_roundtrip, call_span)?);
    }
    *node.ensure_children() = child_doc;
    Ok(node)
}

fn record_to_jik_node(
    record: &Record,
    name: &str,
    non_roundtrip: &NonRoundtrip,
    call_span: Span,
) -> Result<KdlNode, ShellError> {
    let mut node = KdlNode::new(identifier_for(name));

    if record.is_empty() {
        node.set_ty(identifier_for(TY_OBJECT));
        return Ok(node);
    }

    let mut child_doc = KdlDocument::new();
    let mut has_children = false;

    for (key, value) in record.iter() {
        if is_kdl_literal(value) {
            node.push(entry_from_nu_value(
                value,
                Some(key),
                non_roundtrip,
                call_span,
            )?);
        } else {
            has_children = true;
            child_doc
                .nodes_mut()
                .push(value_to_jik_node(value, key, non_roundtrip, call_span)?);
        }
    }

    if has_children {
        *node.ensure_children() = child_doc;
    }

    Ok(node)
}
