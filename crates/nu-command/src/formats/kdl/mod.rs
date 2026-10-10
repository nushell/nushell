//! Shared KDL conversion logic for `from kdl` / `to kdl`.
//!
//! Two data models:
//! - **nodes** — KDL document AST as node rows (`name`/`args`/`props`/`children`/`type`)
//! - **jik** — JSON-in-KDL mapping of ordinary Nushell values
//!
//! Language versions are selected with `--spec 1|2` (default 2).

mod jik;
mod nodes;
mod types;

pub(crate) use jik::{jik_document_to_value, value_to_jik_document};
pub(crate) use nodes::{node_rows_to_kdl_document, nodes_document_to_value};
pub(crate) use types::NonRoundtrip;

use kdl::{KdlDocument, KdlError};
use nu_engine::command_prelude::*;
use nu_protocol::{
    DEFAULT_ERROR_CONTEXT, PipelineMetadata, shell_error::generic::GenericError,
    truncated_source_window,
};

/// Private metadata key written by `from kdl` so `to kdl` can restore format/spec defaults.
pub(crate) const KDL_META_KEY: &str = "nu_kdl";

/// Schema version embedded in metadata.
const KDL_META_VERSION: &str = "v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum KdlSpec {
    V1,
    #[default]
    V2,
}

impl KdlSpec {
    pub(crate) fn from_i64(value: i64, span: Span) -> Result<Self, ShellError> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => Err(ShellError::IncorrectValue {
                msg: "KDL --spec must be 1 or 2".into(),
                val_span: span,
                call_span: span,
            }),
        }
    }

    pub(crate) fn as_i64(self) -> i64 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KdlFormat {
    Nodes,
    Jik,
}

impl KdlFormat {
    pub(crate) fn parse(value: &str, span: Span) -> Result<Self, ShellError> {
        match value {
            "nodes" => Ok(Self::Nodes),
            "jik" => Ok(Self::Jik),
            _ => Err(ShellError::IncorrectValue {
                msg: "KDL --format must be 'nodes' or 'jik'".into(),
                val_span: span,
                call_span: span,
            }),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Nodes => "nodes",
            Self::Jik => "jik",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct KdlMetadata {
    pub format: KdlFormat,
    pub spec: KdlSpec,
}

impl KdlMetadata {
    pub(crate) fn write_to(self, metadata: &mut PipelineMetadata, span: Span) {
        metadata.custom.insert(
            KDL_META_KEY,
            Value::record(
                record! {
                    "version" => Value::string(KDL_META_VERSION, span),
                    "format" => Value::string(self.format.as_str(), span),
                    "spec" => Value::int(self.spec.as_i64(), span),
                },
                span,
            ),
        );
    }

    pub(crate) fn read_from(metadata: &PipelineMetadata) -> Option<Self> {
        let value = metadata.custom.get(KDL_META_KEY)?;
        let record = value.as_record().ok()?;
        let version = record.get("version")?.as_str().ok()?;
        if version != KDL_META_VERSION {
            return None;
        }
        let format =
            KdlFormat::parse(record.get("format")?.as_str().ok()?, Span::unknown()).ok()?;
        let spec = KdlSpec::from_i64(record.get("spec")?.as_int().ok()?, Span::unknown()).ok()?;
        Some(Self { format, spec })
    }

    pub(crate) fn clear(metadata: &mut PipelineMetadata) {
        metadata.custom.remove(KDL_META_KEY);
        // Drop legacy marker from older from kdl implementations.
        metadata.custom.remove("nu_kdl_canonical");
    }
}

pub(crate) fn parse_kdl_document(
    input: &str,
    spec: KdlSpec,
    span: Span,
) -> Result<KdlDocument, ShellError> {
    let result = match spec {
        KdlSpec::V1 => KdlDocument::parse_v1(input),
        KdlSpec::V2 => KdlDocument::parse(input),
    };
    result.map_err(|err| kdl_error_to_shell_error(input, span, &err))
}

pub(crate) fn document_to_string(mut document: KdlDocument, spec: KdlSpec) -> String {
    document.autoformat();
    match spec {
        KdlSpec::V1 => {
            document.ensure_v1();
            document.to_string()
        }
        KdlSpec::V2 => {
            document.ensure_v2();
            document.to_string()
        }
    }
}

pub(crate) fn kdl_error_to_shell_error(input: &str, span: Span, err: &KdlError) -> ShellError {
    if let Some(diagnostic) = err.diagnostics.first() {
        let diagnostic_message = kdl_diagnostics_message(&err.diagnostics);
        let byte_offset = diagnostic.span.offset();
        let (src, label_span) = truncated_source_window(
            input,
            Span::new(byte_offset, byte_offset),
            DEFAULT_ERROR_CONTEXT,
        );

        return ShellError::Generic(
            GenericError::new(
                "Error while parsing KDL text",
                "error parsing KDL text",
                span,
            )
            .with_inner([ShellError::OutsideSpannedLabeledError {
                src,
                error: "Error while parsing KDL text".into(),
                msg: diagnostic_message,
                span: label_span,
            }]),
        );
    }

    ShellError::CantConvert {
        to_type: format!("structured kdl data ({err})"),
        from_type: "string".into(),
        span,
        help: None,
    }
}

pub(crate) fn kdl_diagnostics_message(diagnostics: &[kdl::KdlDiagnostic]) -> String {
    if diagnostics.len() == 1 {
        return kdl_diagnostic_message(&diagnostics[0]);
    }

    diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            format!(
                "diagnostic {}:\n{}",
                index + 1,
                kdl_diagnostic_message(diagnostic)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn kdl_diagnostic_message(diagnostic: &kdl::KdlDiagnostic) -> String {
    let mut parts = Vec::new();

    if let Some(message) = &diagnostic.message {
        parts.push(message.clone());
    }

    if let Some(label) = &diagnostic.label
        && parts.last() != Some(label)
    {
        parts.push(label.clone());
    }

    if let Some(help) = &diagnostic.help {
        parts.push(format!("help: {help}"));
    }

    if parts.is_empty() {
        "error parsing KDL text".to_owned()
    } else {
        parts.join("\n")
    }
}

pub(crate) fn resolve_non_roundtrip(
    serialize: bool,
    non_roundtrip: Option<Spanned<String>>,
    engine_state: &EngineState,
) -> Result<NonRoundtrip, ShellError> {
    match (serialize, non_roundtrip.as_ref().map(|nr| nr.item.as_str())) {
        (false, None | Some("error")) => Ok(NonRoundtrip::Error),
        (true, None | Some("lossy")) => Ok(NonRoundtrip::Lossy {
            engine_state: Box::new(engine_state.clone()),
        }),
        (false, Some("null")) => Ok(NonRoundtrip::Null),
        (false, Some("lossy")) => Ok(NonRoundtrip::Lossy {
            engine_state: Box::new(engine_state.clone()),
        }),
        (false, Some(_)) => Err(ShellError::IncompatibleParametersSingle {
            msg: "expected `error`, `null` or `lossy`".into(),
            span: non_roundtrip.expect("non_roundtrip is some").span,
        }),
        (true, Some(_)) => Err(ShellError::IncompatibleParameters {
            left_message: "this is a shorthand to".into(),
            left_span: Span::unknown(),
            right_message: "this with `lossy`".into(),
            right_span: non_roundtrip.expect("non_roundtrip is some").span,
        }),
    }
}

/// Identifier helper that clears inherited formatting.
pub(crate) fn identifier_for(key: &str) -> kdl::KdlIdentifier {
    let mut identifier = kdl::KdlIdentifier::from(key.to_owned());
    identifier.clear_format();
    identifier
}

#[cfg(test)]
mod spec_tests {
    use super::*;
    use crate::formats::kdl::{
        document_to_string, jik_document_to_value, node_rows_to_kdl_document,
        nodes_document_to_value, parse_kdl_document, value_to_jik_document,
    };
    use nu_protocol::PipelineMetadata;
    use types::NonRoundtrip;

    #[test]
    fn invalid_spec_is_rejected() {
        let err = KdlSpec::from_i64(0, Span::test_data()).expect_err("0 invalid");
        assert!(matches!(err, ShellError::IncorrectValue { .. }));
        let err = KdlSpec::from_i64(3, Span::test_data()).expect_err("3 invalid");
        assert!(matches!(err, ShellError::IncorrectValue { .. }));
        assert_eq!(
            KdlSpec::from_i64(1, Span::test_data()).unwrap(),
            KdlSpec::V1
        );
        assert_eq!(
            KdlSpec::from_i64(2, Span::test_data()).unwrap(),
            KdlSpec::V2
        );
        assert_eq!(KdlSpec::default(), KdlSpec::V2);
    }

    #[test]
    fn v2_parses_hash_keywords_and_rejects_bare_true_property() {
        let span = Span::test_data();
        parse_kdl_document("node #true #false #null", KdlSpec::V2, span)
            .expect("v2 hash keywords should parse");
        parse_kdl_document("node flag=#true", KdlSpec::V2, span).expect("v2 #true prop");
        assert!(
            parse_kdl_document("node flag=true", KdlSpec::V2, span).is_err(),
            "v2 must not accept bare true as boolean property"
        );
        assert!(
            parse_kdl_document("node true false null", KdlSpec::V2, span).is_err(),
            "v2 must not accept bare true/false/null as keyword args"
        );
    }

    #[test]
    fn v1_parses_bare_keywords_and_rejects_hash_keywords() {
        let span = Span::test_data();
        parse_kdl_document("node true false null", KdlSpec::V1, span)
            .expect("v1 bare keywords should parse");
        parse_kdl_document("node flag=true", KdlSpec::V1, span).expect("v1 bare true prop");
        assert!(
            parse_kdl_document("node flag=#true", KdlSpec::V1, span).is_err(),
            "v1 must not accept #true"
        );
        assert!(
            parse_kdl_document("node #true #false #null", KdlSpec::V1, span).is_err(),
            "v1 must not accept #true/#false/#null args"
        );
    }

    #[test]
    fn v1_and_v2_parse_to_equivalent_node_values_for_keywords() {
        let span = Span::test_data();
        let v1 = parse_kdl_document("node true false null", KdlSpec::V1, span).unwrap();
        let v2 = parse_kdl_document("node #true #false #null", KdlSpec::V2, span).unwrap();
        let rows_v1 = nodes_document_to_value(&v1, span, false).unwrap();
        let rows_v2 = nodes_document_to_value(&v2, span, false).unwrap();
        assert_eq!(rows_v1, rows_v2);
    }

    #[test]
    fn emit_v1_vs_v2_keyword_spelling_for_jik() {
        let span = Span::test_data();
        let value = Value::test_record(record! {
            "a" => Value::int(1, span),
            "b" => Value::bool(true, span),
            "c" => Value::nothing(span),
            "d" => Value::bool(false, span),
        });
        let doc = value_to_jik_document(&value, &NonRoundtrip::Error, span).unwrap();
        let v2 = document_to_string(doc.clone(), KdlSpec::V2);
        let v1 = document_to_string(doc, KdlSpec::V1);
        assert_eq!(v2, "- a=1 b=#true c=#null d=#false\n");
        assert_eq!(v1, "- a=1 b=true c=null d=false\n");
    }

    #[test]
    fn emit_v1_vs_v2_keyword_spelling_for_nodes() {
        let span = Span::test_data();
        let rows = Value::test_list(vec![Value::test_record(record! {
            "name" => Value::string("item", span),
            "args" => Value::test_list(vec![
                Value::bool(true, span),
                Value::bool(false, span),
                Value::nothing(span),
            ]),
            "props" => Value::test_record(record! {
                "enabled" => Value::bool(true, span),
            }),
            "children" => Value::test_list(vec![]),
        })]);
        let doc = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(
            document_to_string(doc.clone(), KdlSpec::V2),
            "item #true #false #null enabled=#true\n"
        );
        assert_eq!(
            document_to_string(doc, KdlSpec::V1),
            "item true false null enabled=true\n"
        );
    }

    #[test]
    fn jik_round_trip_v2_text() {
        let span = Span::test_data();
        let original = Value::test_record(record! {
            "name" => Value::string("nu", span),
            "ok" => Value::bool(true, span),
            "count" => Value::int(3, span),
        });
        let doc = value_to_jik_document(&original, &NonRoundtrip::Error, span).unwrap();
        let text = document_to_string(doc, KdlSpec::V2);
        let parsed = parse_kdl_document(&text, KdlSpec::V2, span).unwrap();
        let back = jik_document_to_value(&parsed, span, false).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn jik_round_trip_v1_text() {
        let span = Span::test_data();
        let original = Value::test_record(record! {
            "name" => Value::string("nu", span),
            "ok" => Value::bool(true, span),
            "count" => Value::int(3, span),
        });
        let doc = value_to_jik_document(&original, &NonRoundtrip::Error, span).unwrap();
        let text = document_to_string(doc, KdlSpec::V1);
        assert!(
            text.contains("ok=true"),
            "v1 emit should use bare true: {text}"
        );
        assert!(
            !text.contains("#true"),
            "v1 emit should not use #true: {text}"
        );
        let parsed = parse_kdl_document(&text, KdlSpec::V1, span).unwrap();
        let back = jik_document_to_value(&parsed, span, false).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn nodes_round_trip_v1_and_v2_text() {
        let span = Span::test_data();
        // Flat nodes avoid v1 nested bare-identifier quirks; keyword spelling is what we care about.
        let v1_src = "item 1 enabled=true\n";
        let v2_src = "item 1 enabled=#true\n";

        let v1_doc = parse_kdl_document(v1_src, KdlSpec::V1, span).unwrap();
        let rows = nodes_document_to_value(&v1_doc, span, false).unwrap();
        let rebuilt = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(document_to_string(rebuilt.clone(), KdlSpec::V1), v1_src);
        assert_eq!(document_to_string(rebuilt, KdlSpec::V2), v2_src);

        let v2_doc = parse_kdl_document(v2_src, KdlSpec::V2, span).unwrap();
        let rows2 = nodes_document_to_value(&v2_doc, span, false).unwrap();
        assert_eq!(rows, rows2);
    }

    #[test]
    fn nested_v2_document_round_trips() {
        let span = Span::test_data();
        let v2_src = "package {\n    name nu\n    enabled #true\n}\n";
        let doc = parse_kdl_document(v2_src, KdlSpec::V2, span).unwrap();
        let rows = nodes_document_to_value(&doc, span, false).unwrap();
        let rebuilt = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(document_to_string(rebuilt, KdlSpec::V2), v2_src);
    }

    #[test]
    fn nested_v1_document_with_quoted_string_args_round_trips() {
        let span = Span::test_data();
        // KDL v1 often needs quoted strings for values that v2 can leave bare.
        let v1_src = "package {\n    name \"nu\"\n    enabled true\n}\n";
        let doc = parse_kdl_document(v1_src, KdlSpec::V1, span).unwrap();
        let rows = nodes_document_to_value(&doc, span, false).unwrap();
        let rebuilt = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        let emitted = document_to_string(rebuilt, KdlSpec::V1);
        // Re-parse emitted v1 text to confirm it is valid and value-equivalent.
        let again = parse_kdl_document(&emitted, KdlSpec::V1, span).unwrap();
        let rows2 = nodes_document_to_value(&again, span, false).unwrap();
        assert_eq!(rows, rows2);
        assert!(
            emitted.contains("enabled true") || emitted.contains("enabled=true"),
            "{emitted}"
        );
    }

    #[test]
    fn cross_spec_parse_then_emit_converts_keywords() {
        let span = Span::test_data();
        // Parse v1, emit v2
        let doc = parse_kdl_document("item 1 enabled=true", KdlSpec::V1, span).unwrap();
        let rows = nodes_document_to_value(&doc, span, false).unwrap();
        let rebuilt = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(
            document_to_string(rebuilt, KdlSpec::V2),
            "item 1 enabled=#true\n"
        );

        // Parse v2, emit v1
        let doc = parse_kdl_document("item 1 enabled=#true", KdlSpec::V2, span).unwrap();
        let rows = nodes_document_to_value(&doc, span, false).unwrap();
        let rebuilt = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(
            document_to_string(rebuilt, KdlSpec::V1),
            "item 1 enabled=true\n"
        );
    }

    #[test]
    fn jik_v1_and_v2_text_parse_to_same_value() {
        let span = Span::test_data();
        let v1 = parse_kdl_document("- a=1 b=true c=null", KdlSpec::V1, span).unwrap();
        let v2 = parse_kdl_document("- a=1 b=#true c=#null", KdlSpec::V2, span).unwrap();
        let val1 = jik_document_to_value(&v1, span, false).unwrap();
        let val2 = jik_document_to_value(&v2, span, false).unwrap();
        assert_eq!(val1, val2);
        assert_eq!(
            val1,
            Value::test_record(record! {
                "a" => Value::int(1, span),
                "b" => Value::bool(true, span),
                "c" => Value::nothing(span),
            })
        );
    }

    #[test]
    fn nested_structure_emits_consistently_across_specs() {
        let span = Span::test_data();
        let value = Value::test_record(record! {
            "meta" => Value::test_record(record! {
                "active" => Value::bool(true, span),
            }),
            "tags" => Value::test_list(vec![
                Value::string("a", span),
                Value::string("b", span),
            ]),
        });
        let doc = value_to_jik_document(&value, &NonRoundtrip::Error, span).unwrap();
        let v1 = document_to_string(doc.clone(), KdlSpec::V1);
        let v2 = document_to_string(doc, KdlSpec::V2);
        // Structure should match; only keyword spelling differs.
        assert!(
            v2.contains("active=#true") || v2.contains("#true"),
            "v2: {v2}"
        );
        assert!(
            v1.contains("active=true") && !v1.contains("#true"),
            "v1: {v1}"
        );
        let back1 = jik_document_to_value(
            &parse_kdl_document(&v1, KdlSpec::V1, span).unwrap(),
            span,
            false,
        )
        .unwrap();
        let back2 = jik_document_to_value(
            &parse_kdl_document(&v2, KdlSpec::V2, span).unwrap(),
            span,
            false,
        )
        .unwrap();
        assert_eq!(back1, value);
        assert_eq!(back2, value);
    }

    #[test]
    fn metadata_preserves_spec_one_and_two() {
        let span = Span::test_data();
        for spec in [KdlSpec::V1, KdlSpec::V2] {
            let mut metadata = PipelineMetadata::default();
            KdlMetadata {
                format: KdlFormat::Nodes,
                spec,
            }
            .write_to(&mut metadata, span);
            let read = KdlMetadata::read_from(&metadata).expect("read");
            assert_eq!(read.spec, spec);
            assert_eq!(read.format, KdlFormat::Nodes);
        }
    }

    #[test]
    fn list_of_bools_emits_keyword_args_per_spec() {
        let span = Span::test_data();
        let value = Value::test_list(vec![
            Value::nothing(span),
            Value::bool(false, span),
            Value::bool(true, span),
        ]);
        let doc = value_to_jik_document(&value, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(
            document_to_string(doc.clone(), KdlSpec::V2),
            "- #null #false #true\n"
        );
        assert_eq!(document_to_string(doc, KdlSpec::V1), "- null false true\n");
    }
}
