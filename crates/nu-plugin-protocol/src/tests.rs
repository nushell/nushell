use super::*;
use nu_protocol::{
    BuiltinCompletion, Category, CommandWideCompleter, Completion, DataSource, Filesize,
    HistoryFileFormat, OneOf, PluginExample, Range, Signature, SuggestionKind, SyntaxShape, Type,
    VarId,
    ast::{Assignment, Bits, Boolean, CellPath, Comparison, Math, PathMember},
    engine::CommandType,
    record,
};
use serde::de::DeserializeOwned;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Protocol info with the given protocol version and nothing else.
fn info(version: &str) -> ProtocolInfo {
    ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: version.into(),
        nushell_version: None,
        features: vec![],
    }
}

#[test]
fn protocol_info_compatible() -> Result<(), ShellError> {
    assert!(info("1.1.0").is_compatible_with(&info("1.2.3"))?);
    assert!(info("1.2.3").is_compatible_with(&info("1.1.0"))?);
    Ok(())
}

#[test]
fn protocol_info_incompatible() -> Result<(), ShellError> {
    assert!(!info("2.0.0").is_compatible_with(&info("1.1.0"))?);
    assert!(!info("1.1.0").is_compatible_with(&info("2.0.0"))?);
    Ok(())
}

#[test]
fn protocol_info_compatible_with_nightly() -> Result<(), ShellError> {
    assert!(info("1.1.0-nightly.1").is_compatible_with(&info("1.2.3"))?);
    assert!(info("1.2.3").is_compatible_with(&info("1.1.0-nightly.1"))?);
    Ok(())
}

#[test]
fn protocol_info_0x_is_compatible_within_a_minor_line_only() -> Result<(), ShellError> {
    assert!(info("0.1.0").is_compatible_with(&info("0.1.1"))?);
    assert!(!info("0.1.0").is_compatible_with(&info("0.2.0"))?);
    // A plugin built before the protocol had its own version reports the Nushell version.
    assert!(!info(PLUGIN_PROTOCOL_VERSION).is_compatible_with(&info("0.116.2"))?);
    Ok(())
}

#[test]
fn protocol_info_default_reports_protocol_and_nushell_versions() {
    let default = ProtocolInfo::default();
    assert_eq!(default.version, PLUGIN_PROTOCOL_VERSION);
    assert_eq!(
        default.nushell_version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn same_nushell_version_requires_both_versions_to_match() {
    let built_from = |version: Option<&str>| ProtocolInfo {
        nushell_version: version.map(Into::into),
        ..info(PLUGIN_PROTOCOL_VERSION)
    };
    assert!(built_from(Some("0.116.2")).same_nushell_version(&built_from(Some("0.116.2"))));
    assert!(!built_from(Some("0.116.2")).same_nushell_version(&built_from(Some("0.117.0"))));
    assert!(!built_from(Some("0.116.2")).same_nushell_version(&built_from(None)));
    assert!(!built_from(None).same_nushell_version(&built_from(None)));
}

#[test]
fn hello_without_nushell_version_decodes() {
    // Plugins not written in Rust send no Nushell version.
    let hello: PluginOutput = serde_json::from_str(
        r#"{"Hello": {"protocol": "nu-plugin", "version": "0.1.0", "features": []}}"#,
    )
    .expect("hello without nushell_version should decode");
    let PluginOutput::Hello(info) = hello else {
        panic!("expected Hello, got {hello:?}");
    };
    assert_eq!(info.nushell_version, None);
}

#[test]
fn unknown_feature_cannot_serialize() {
    assert!(serde_json::to_value(Feature::Unknown).is_err());
}

// Config travels as a record so that config changes between Nushell versions stay compatible.

/// Decode an [`EngineCallResponse::Config`] whose payload is `value`.
fn decode_config(value: Value) -> Result<Config, serde_json::Error> {
    let payload = serde_json::to_value(value).expect("value should serialize");
    match serde_json::from_value(serde_json::json!({ "Config": payload }))? {
        EngineCallResponse::<PipelineDataHeader>::Config(config) => Ok(Config::clone(&config)),
        other => panic!("expected Config, got {other:?}"),
    }
}

/// The config as JSON, for comparing configs (`Config` has no `PartialEq`).
fn config_json(config: &Config) -> serde_json::Value {
    serde_json::to_value(config).expect("config should serialize")
}

#[test]
fn config_record_roundtrips() {
    let mut config = Config::default();
    config.float_precision = 7;
    config.show_hints = !config.show_hints;
    // Converting the config to a record gives nested values the record's unknown span.
    config
        .plugins
        .insert("example".into(), Value::string("setting", Span::unknown()));
    // Fewer keybindings than the defaults: the removed defaults must not come back.
    config.keybindings.truncate(1);
    // This crate's tests build nu-protocol without its `sqlite` feature, like a plugin built
    // outside the workspace, whose config parser would turn this into plaintext.
    config.history.file_format = HistoryFileFormat::Sqlite;

    let response = EngineCallResponse::<PipelineDataHeader>::Config(SharedCow::new(config.clone()));
    let json = serde_json::to_value(&response).expect("config response should serialize");
    let EngineCallResponse::Config(decoded) =
        serde_json::from_value::<EngineCallResponse<PipelineDataHeader>>(json)
            .expect("config response should decode")
    else {
        panic!("expected a Config response");
    };

    assert_eq!(config_json(&decoded), config_json(&config));
}

#[test]
fn config_record_tolerates_unknown_and_missing_options() {
    let config = decode_config(Value::test_record(record! {
        "float_precision" => Value::test_int(3),
        "an_option_from_a_newer_nushell" => Value::test_bool(true),
        "footer_mode" => Value::test_string("a_mode_from_a_newer_nushell"),
    }))
    .expect("a config record should always decode");

    let expected = Config {
        float_precision: 3,
        ..Config::default()
    };
    assert_eq!(config_json(&config), config_json(&expected));
}

#[test]
fn config_must_be_a_record() {
    assert!(decode_config(Value::test_int(1)).is_err());
}

// The corpus below holds every frozen message and every variant of the frozen enums. It is
// checked two ways:
//
// - `messages_match_snapshots` compares the current encoding, as text, with
//   `protocol_snapshots/*.json`, so any change to the wire shows up in review, including the
//   order of keys and record columns.
// - `frozen_fixtures_decode` decodes `protocol_snapshots/frozen/<version>/*.json`, which are
//   written once per protocol version and never regenerated, and encodes them again. Messages
//   from older compatible peers must keep decoding, and must still encode to what those peers
//   read.
//
// Engine-internal payloads (the IR in `EngineCallResponse::IrBlock`, the parsed call in
// `PluginCall::GetCompletion`) and the tolerant `EngineCallResponse::Config` are left out; see
// `PLUGIN_PROTOCOL_VERSION`.

fn span(start: usize) -> Span {
    Span::new(start, start + 1)
}

fn labeled_error() -> LabeledError {
    LabeledError::new("error message")
        .with_label("label", span(1))
        .with_code("nu::test::error")
        .with_url("https://www.nushell.sh")
        .with_help("help text")
        .with_inner(LabeledError::new("inner error"))
}

fn plugin_custom_value(notify_on_drop: bool) -> PluginCustomValue {
    PluginCustomValue::new("CustomType".into(), vec![1, 2, 3], notify_on_drop)
}

/// One value of every [`Value`] variant.
fn every_value() -> Vec<Value> {
    let date =
        chrono::DateTime::parse_from_rfc3339("2024-01-02T03:04:05+02:00").expect("valid date");
    let range = "1..3..10".parse::<Range>().expect("valid range");
    vec![
        Value::bool(true, span(1)),
        Value::int(-7, span(2)),
        Value::float(1.5, span(3)),
        Value::string("text", span(4)),
        Value::glob("*.rs", true, span(5)),
        Value::filesize(Filesize::new(1024), span(6)),
        Value::duration(1_000_000_000, span(7)),
        Value::date(date, span(8)),
        Value::range(range, span(9)),
        Value::record(
            record! {
                "a" => Value::int(1, span(10)),
                "b" => Value::list(vec![Value::nothing(span(11))], span(12)),
            },
            span(13),
        ),
        Value::list(
            vec![Value::int(1, span(14)), Value::string("two", span(15))],
            span(16),
        ),
        Value::closure(
            Closure {
                block_id: BlockId::new(3),
                captures: vec![(VarId::new(4), Value::int(5, span(17)))],
            },
            span(18),
        ),
        Value::error(labeled_error().into(), span(19)),
        Value::binary(vec![0, 1, 255], span(20)),
        Value::cell_path(
            CellPath {
                members: vec![
                    PathMember::String {
                        val: "name".into(),
                        span: span(21),
                        optional: true,
                        casing: Casing::Insensitive,
                    },
                    PathMember::Int {
                        val: 2,
                        span: span(22),
                        optional: false,
                    },
                ],
            },
            span(23),
        ),
        Value::custom(Box::new(plugin_custom_value(true)), span(24)),
        Value::nothing(span(25)),
    ]
}

/// `Range` and `CellPath` travel as their display strings, which share formatting and quoting
/// code with `to nuon`. One value for each form those strings take, so that a change to that
/// code shows up in the snapshots.
fn display_string_values() -> Vec<Value> {
    let ranges = [
        "1..10",
        "1..<10",
        "5..",
        "-5..-1",
        "10..8..0",
        "1.5..3.0",
        "0.5..1.0..<3.0",
        "0.5..",
    ]
    .into_iter()
    .map(|range| Value::range(range.parse::<Range>().expect("valid range"), span(1)));
    let member = |val: &str| PathMember::String {
        val: val.into(),
        span: span(2),
        optional: false,
        casing: Casing::Sensitive,
    };
    let cell_paths = [
        // Members that need quotes.
        ["a b", "1", "true", "inf", "x.y", "a=b", "a?", "b!", "#c"]
            .map(member)
            .to_vec(),
        ["a\"b", "a\\b", "a\0b", ""].map(member).to_vec(),
        // Optional members of both kinds.
        vec![
            PathMember::Int {
                val: 0,
                span: span(3),
                optional: true,
            },
            PathMember::String {
                val: "c".into(),
                span: span(4),
                optional: true,
                casing: Casing::Sensitive,
            },
        ],
        // The empty path.
        vec![],
    ]
    .into_iter()
    .map(|members| Value::cell_path(CellPath { members }, span(5)));
    ranges.chain(cell_paths).collect()
}

fn every_syntax_shape() -> Vec<SyntaxShape> {
    vec![
        SyntaxShape::Any,
        SyntaxShape::Binary,
        SyntaxShape::Block,
        SyntaxShape::Boolean,
        SyntaxShape::CellPath,
        SyntaxShape::Closure(Some(vec![SyntaxShape::Int])),
        SyntaxShape::DateTime,
        SyntaxShape::Directory,
        SyntaxShape::Duration,
        SyntaxShape::Error,
        SyntaxShape::Expression,
        SyntaxShape::ExternalArgument,
        SyntaxShape::Filepath,
        SyntaxShape::Filesize,
        SyntaxShape::Float,
        SyntaxShape::FullCellPath,
        SyntaxShape::GlobPattern,
        SyntaxShape::Int,
        SyntaxShape::ImportPattern,
        SyntaxShape::Keyword(b"as".to_vec(), Box::new(SyntaxShape::String)),
        SyntaxShape::List(Box::new(SyntaxShape::Number)),
        SyntaxShape::MathExpression,
        SyntaxShape::MatchBlock,
        SyntaxShape::Nothing,
        SyntaxShape::Number,
        SyntaxShape::OneOf(vec![SyntaxShape::Int, SyntaxShape::String]),
        SyntaxShape::Operator,
        SyntaxShape::Range,
        SyntaxShape::Record([("a", SyntaxShape::Int)].into()),
        SyntaxShape::RowCondition,
        SyntaxShape::Signature,
        SyntaxShape::ExternalSignature,
        SyntaxShape::String,
        SyntaxShape::Table([("b", SyntaxShape::String)].into()),
        SyntaxShape::VarWithOptType,
    ]
}

fn every_type() -> Vec<Type> {
    vec![
        Type::Any,
        Type::Binary,
        Type::Block,
        Type::Bool,
        Type::CellPath,
        Type::Closure,
        Type::Custom("CustomType".into()),
        Type::Date,
        Type::Duration,
        Type::Error,
        Type::Filesize,
        Type::Float,
        Type::Int,
        Type::List(Box::new(Type::String)),
        Type::Nothing,
        Type::Number,
        Type::OneOf(OneOf::default().add_ty(Type::Int).add_ty(Type::String)),
        Type::Range,
        Type::Record([("a", Type::Int)].into()),
        Type::String,
        Type::Glob,
        Type::Table([("b", Type::String)].into()),
    ]
}

/// A signature that sets every field and uses every [`SyntaxShape`] and [`Type`].
fn full_signature() -> PluginSignature {
    let mut sig = Signature::build("demo")
        .description("description")
        .extra_description("extra description")
        .search_terms(vec!["term".into()])
        .required("required", SyntaxShape::String, "required positional")
        .named("named", SyntaxShape::Int, "named flag", Some('n'))
        .switch("switch", "switch flag", None)
        .rest("rest", SyntaxShape::Any, "rest positional")
        .input_output_types(
            every_type()
                .into_iter()
                .map(|ty| (ty.clone(), ty))
                .collect(),
        )
        .category(Category::Custom("plugin category".into()));
    for (index, shape) in every_syntax_shape().into_iter().enumerate() {
        sig = sig.optional(format!("optional{index}"), shape, "optional positional");
    }
    sig.allow_variants_without_examples = true;
    sig.is_filter = true;
    sig.creates_scope = true;
    sig.allows_unknown_args = true;
    sig.complete = Some(CommandWideCompleter::External);
    sig.required_positional[0].completion = Some(Completion::new_list(&["a", "b"]));
    sig.required_positional[0].default_value = Some(Value::string("a", span(1)));
    sig.optional_positional[0].completion = Some(Completion::Command(DeclId::new(7)));
    sig.optional_positional[1].completion = Some(Completion::Builtin(BuiltinCompletion::NuFile {
        std_virtual_path: true,
    }));
    sig.optional_positional[2].completion = Some(Completion::Builtin(BuiltinCompletion::Command {
        internal_only: false,
    }));
    sig.optional_positional[3].completion = Some(Completion::Builtin(BuiltinCompletion::EnvVar));
    sig.optional_positional[4].completion =
        Some(Completion::Builtin(BuiltinCompletion::ModuleExports));
    if let Some(flag) = sig.named.iter_mut().find(|flag| flag.long == "named") {
        flag.completion = Some(Completion::new_list(&["1", "2"]));
        flag.var_id = Some(VarId::new(8));
        flag.default_value = Some(Value::int(1, span(2)));
    }

    PluginSignature::new(
        sig,
        vec![PluginExample {
            example: "demo a".into(),
            description: "example".into(),
            result: Some(Value::string("result", span(3))),
        }],
    )
}

fn suggestions() -> Vec<DynamicSuggestion> {
    vec![
        DynamicSuggestion {
            value: "value".into(),
            display_override: Some("display".into()),
            description: Some("description".into()),
            extra: Some(vec!["extra".into()]),
            append_whitespace: true,
            match_indices: Some(vec![0, 2]),
            span: Some(span(1)),
            kind: Some(SuggestionKind::Command(
                CommandType::Plugin,
                Some(DeclId::new(9)),
            )),
        },
        DynamicSuggestion {
            value: "other".into(),
            kind: Some(SuggestionKind::Value(Type::Int)),
            ..DynamicSuggestion::default()
        },
        DynamicSuggestion {
            value: "file".into(),
            kind: Some(SuggestionKind::File),
            ..DynamicSuggestion::default()
        },
    ]
}

fn full_metadata() -> PipelineMetadata {
    PipelineMetadata {
        data_source: DataSource::FilePath(PathBuf::from("/data/file.json")),
        path_columns: vec!["name".into()],
        content_type: Some("application/json".into()),
        custom: record! { "plugin_field" => Value::string("x", span(1)) },
    }
}

fn hello() -> ProtocolInfo {
    ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: PLUGIN_PROTOCOL_VERSION.into(),
        // Fixed so that the snapshots don't change with every Nushell release.
        nushell_version: Some("0.0.0-test".into()),
        features: vec![Feature::LocalSocket],
    }
}

fn evaluated_call() -> EvaluatedCall {
    EvaluatedCall {
        head: span(1),
        positional: every_value(),
        named: vec![
            (
                Spanned {
                    item: "named".into(),
                    span: span(2),
                },
                Some(Value::int(9, span(3))),
            ),
            (
                Spanned {
                    item: "switch".into(),
                    span: span(4),
                },
                None,
            ),
        ],
    }
}

fn run(input: PipelineDataHeader) -> PluginCall<PipelineDataHeader> {
    PluginCall::Run(CallInfo {
        name: "demo".into(),
        call: evaluated_call(),
        input,
    })
}

fn custom_value_op(op: CustomValueOp) -> PluginCall<PipelineDataHeader> {
    PluginCall::CustomValueOp(
        Spanned {
            item: plugin_custom_value(false),
            span: span(1),
        },
        op,
    )
}

/// Every [`PluginInput`] message in the frozen tier.
fn plugin_input_messages() -> Vec<PluginInput> {
    let calls = [
        PluginCall::Metadata,
        PluginCall::Signature,
        run(PipelineDataHeader::Empty),
        run(PipelineDataHeader::Value(
            Value::string("input", span(1)),
            Some(full_metadata()),
        )),
        run(PipelineDataHeader::ListStream(ListStreamInfo {
            id: 0,
            span: span(2),
            metadata: Some(PipelineMetadata {
                data_source: DataSource::HtmlThemes,
                ..PipelineMetadata::default()
            }),
        })),
        run(PipelineDataHeader::ByteStream(ByteStreamInfo {
            id: 1,
            span: span(3),
            type_: ByteStreamType::String,
            metadata: Some(PipelineMetadata::default()),
        })),
        custom_value_op(CustomValueOp::ToBaseValue),
        custom_value_op(CustomValueOp::FollowPathInt {
            index: Spanned {
                item: 0,
                span: span(4),
            },
            optional: true,
        }),
        custom_value_op(CustomValueOp::FollowPathString {
            column_name: Spanned {
                item: "column".into(),
                span: span(5),
            },
            optional: false,
            casing: Casing::Sensitive,
        }),
        custom_value_op(CustomValueOp::PartialCmp(Value::int(1, span(6)))),
        custom_value_op(CustomValueOp::Operation(
            Spanned {
                item: Operator::Math(Math::Add),
                span: span(7),
            },
            Value::int(2, span(8)),
        )),
        custom_value_op(CustomValueOp::Operation(
            Spanned {
                item: Operator::Comparison(Comparison::Equal),
                span: span(9),
            },
            Value::int(3, span(10)),
        )),
        custom_value_op(CustomValueOp::Save {
            path: Spanned {
                item: PathBuf::from("out.bin"),
                span: span(11),
            },
            save_call_span: span(12),
        }),
        custom_value_op(CustomValueOp::Dropped),
    ];
    let responses = [
        EngineCallResponse::Error(labeled_error().into()),
        EngineCallResponse::PipelineData(PipelineDataHeader::ByteStream(ByteStreamInfo {
            id: 2,
            span: span(13),
            type_: ByteStreamType::Binary,
            metadata: None,
        })),
        EngineCallResponse::PipelineData(PipelineDataHeader::ByteStream(ByteStreamInfo {
            id: 3,
            span: span(14),
            type_: ByteStreamType::Unknown,
            metadata: None,
        })),
        EngineCallResponse::ValueMap(HashMap::from([(
            "PWD".to_string(),
            Value::string("/home", span(15)),
        )])),
        EngineCallResponse::Identifier(DeclId::new(5)),
    ];

    let mut messages = vec![PluginInput::Hello(hello())];
    messages.extend(
        calls
            .into_iter()
            .enumerate()
            .map(|(id, call)| PluginInput::Call(id, call)),
    );
    messages.extend(
        responses
            .into_iter()
            .enumerate()
            .map(|(id, response)| PluginInput::EngineCallResponse(id, response)),
    );
    messages.extend([
        PluginInput::Data(0, StreamData::List(Value::int(1, span(16)))),
        PluginInput::Data(
            0,
            StreamData::List(Value::list(display_string_values(), span(17))),
        ),
        PluginInput::Data(1, StreamData::Raw(Ok(vec![1, 2, 3]))),
        PluginInput::Data(1, StreamData::Raw(Err(labeled_error()))),
        PluginInput::End(0),
        PluginInput::Drop(0),
        PluginInput::Ack(0),
        PluginInput::Signal(SignalAction::Interrupt),
        PluginInput::Signal(SignalAction::Reset),
        PluginInput::Goodbye,
    ]);
    messages
}

/// Every [`PluginOutput`] message in the frozen tier.
fn plugin_output_messages() -> Vec<PluginOutput> {
    let responses = [
        PluginCallResponse::Ok,
        PluginCallResponse::Error(labeled_error().into()),
        PluginCallResponse::Metadata(
            PluginMetadata::new()
                .with_version("1.2.3")
                .with_protocol_version(PLUGIN_PROTOCOL_VERSION)
                .with_nushell_version("0.0.0-test"),
        ),
        PluginCallResponse::Signature(vec![full_signature()]),
        PluginCallResponse::Ordering(Some(Ordering::Less)),
        PluginCallResponse::Ordering(Some(Ordering::Equal)),
        PluginCallResponse::Ordering(Some(Ordering::Greater)),
        PluginCallResponse::Ordering(None),
        PluginCallResponse::CompletionItems(Some(suggestions())),
        PluginCallResponse::CompletionItems(None),
        PluginCallResponse::PipelineData(PipelineDataHeader::Value(
            Value::list(every_value(), span(1)),
            Some(full_metadata()),
        )),
    ];
    let engine_calls = [
        EngineCall::GetConfig,
        EngineCall::GetPluginConfig,
        EngineCall::GetEnvVar("PWD".into()),
        EngineCall::GetEnvVars,
        EngineCall::GetCurrentDir,
        EngineCall::AddEnvVar("FOO".into(), Value::string("bar", span(2))),
        EngineCall::GetHelp,
        EngineCall::EnterForeground,
        EngineCall::LeaveForeground,
        EngineCall::GetSpanContents(span(3)),
        EngineCall::EvalClosure {
            closure: Spanned {
                item: Closure {
                    block_id: BlockId::new(1),
                    captures: vec![],
                },
                span: span(4),
            },
            positional: vec![Value::int(1, span(5))],
            input: PipelineDataHeader::Empty,
            redirect_stdout: true,
            redirect_stderr: false,
        },
        EngineCall::FindDecl("ls".into()),
        EngineCall::GetBlockIR(BlockId::new(2)),
        EngineCall::CallDecl {
            decl_id: DeclId::new(3),
            call: evaluated_call(),
            input: PipelineDataHeader::Value(Value::int(1, span(6)), None),
            redirect_stdout: false,
            redirect_stderr: true,
        },
    ];

    let mut messages = vec![
        PluginOutput::Hello(hello()),
        PluginOutput::Option(PluginOption::GcDisabled(true)),
    ];
    messages.extend(
        responses
            .into_iter()
            .enumerate()
            .map(|(id, response)| PluginOutput::CallResponse(id, response)),
    );
    messages.extend(engine_calls.into_iter().enumerate().map(|(id, call)| {
        PluginOutput::EngineCall {
            context: 0,
            id,
            call,
        }
    }));
    messages.extend([
        PluginOutput::Data(0, StreamData::List(Value::string("item", span(7)))),
        PluginOutput::Data(1, StreamData::Raw(Ok(vec![4, 5, 6]))),
        PluginOutput::End(0),
        PluginOutput::Drop(0),
        PluginOutput::Ack(0),
    ]);
    messages
}

/// Every variant of the frozen enums that the messages above only use some of. An enum encodes
/// the same wherever it appears, so one list per enum covers them without a message for each
/// variant.
#[derive(Serialize, Deserialize)]
struct Variants {
    category: Vec<Category>,
    operator: Vec<Operator>,
    casing: Vec<Casing>,
    suggestion_kind: Vec<SuggestionKind>,
    command_type: Vec<CommandType>,
    command_wide_completer: Vec<CommandWideCompleter>,
}

fn every_variant() -> Variants {
    let comparisons = [
        Comparison::Equal,
        Comparison::NotEqual,
        Comparison::LessThan,
        Comparison::GreaterThan,
        Comparison::LessThanOrEqual,
        Comparison::GreaterThanOrEqual,
        Comparison::RegexMatch,
        Comparison::NotRegexMatch,
        Comparison::In,
        Comparison::NotIn,
        Comparison::Has,
        Comparison::NotHas,
        Comparison::StartsWith,
        Comparison::NotStartsWith,
        Comparison::EndsWith,
        Comparison::NotEndsWith,
    ];
    let maths = [
        Math::Add,
        Math::Subtract,
        Math::Multiply,
        Math::Divide,
        Math::FloorDivide,
        Math::Modulo,
        Math::Pow,
        Math::Concatenate,
    ];
    let booleans = [Boolean::Or, Boolean::Xor, Boolean::And];
    let bits = [
        Bits::BitOr,
        Bits::BitXor,
        Bits::BitAnd,
        Bits::ShiftLeft,
        Bits::ShiftRight,
    ];
    let assignments = [
        Assignment::Assign,
        Assignment::AddAssign,
        Assignment::SubtractAssign,
        Assignment::MultiplyAssign,
        Assignment::DivideAssign,
        Assignment::ConcatenateAssign,
    ];

    Variants {
        category: vec![
            Category::Bits,
            Category::Bytes,
            Category::Chart,
            Category::Conversions,
            Category::Core,
            Category::Custom("custom".into()),
            Category::Database,
            Category::Date,
            Category::Debug,
            Category::Default,
            Category::Deprecated,
            Category::Removed,
            Category::Env,
            Category::Experimental,
            Category::FileSystem,
            Category::Filters,
            Category::Formats,
            Category::Generators,
            Category::Hash,
            Category::History,
            Category::Math,
            Category::Misc,
            Category::Network,
            Category::Path,
            Category::Platform,
            Category::Plugin,
            Category::Random,
            Category::Shells,
            Category::Strings,
            Category::System,
            Category::Viewers,
        ],
        operator: (comparisons.map(Operator::Comparison).into_iter())
            .chain(maths.map(Operator::Math))
            .chain(booleans.map(Operator::Boolean))
            .chain(bits.map(Operator::Bits))
            .chain(assignments.map(Operator::Assignment))
            .collect(),
        casing: vec![Casing::Sensitive, Casing::Insensitive],
        suggestion_kind: vec![
            SuggestionKind::Command(CommandType::Builtin, None),
            SuggestionKind::Value(Type::String),
            SuggestionKind::CellPath,
            SuggestionKind::Directory,
            SuggestionKind::File,
            SuggestionKind::Flag,
            SuggestionKind::Module,
            SuggestionKind::Operator,
            SuggestionKind::Variable,
        ],
        command_type: vec![
            CommandType::Builtin,
            CommandType::Custom,
            CommandType::Keyword,
            CommandType::External,
            CommandType::Alias,
            CommandType::Plugin,
        ],
        command_wide_completer: vec![
            CommandWideCompleter::External,
            CommandWideCompleter::Command(DeclId::new(10)),
        ],
    }
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("protocol_snapshots")
}

/// The corpus as pretty JSON, keyed by snapshot file name. Every file holds an array, even
/// `variants.json` with its single entry, so that all of them are checked the same way.
///
/// The text comes straight from the typed values. Going through `serde_json::Value` would sort
/// the keys, or not, depending on whether a workspace build turns on serde_json's
/// `preserve_order` feature.
fn corpus() -> [(&'static str, String); 3] {
    fn to_text<T: Serialize>(entries: &[T]) -> String {
        serde_json::to_string_pretty(entries).expect("corpus should serialize") + "\n"
    }
    [
        ("plugin_input.json", to_text(&plugin_input_messages())),
        ("plugin_output.json", to_text(&plugin_output_messages())),
        ("variants.json", to_text(&[every_variant()])),
    ]
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

#[test]
fn messages_match_snapshots() {
    for (file, actual) in corpus() {
        let expected = read_text(&snapshot_dir().join(file));
        // Comparing lines also ignores the `\r` of a checkout with Windows line endings.
        let actual: Vec<&str> = actual.lines().collect();
        let expected: Vec<&str> = expected.lines().collect();
        if actual != expected {
            let line = actual
                .iter()
                .zip(&expected)
                .take_while(|(actual, expected)| actual == expected)
                .count();
            panic!(
                "protocol_snapshots/{file} changed on the wire at line {}:\n  snapshot: {}\n  \
                 current:  {}\nIf that is intended, decide on a PLUGIN_PROTOCOL_VERSION bump \
                 (see its docs), then run \
                 `cargo test -p nu-plugin-protocol -- --ignored regenerate_protocol_snapshots`",
                line + 1,
                expected.get(line).unwrap_or(&"<end of file>"),
                actual.get(line).unwrap_or(&"<end of file>"),
            );
        }
    }
}

/// True if `encoded` has every key, element, and value that `fixture` has. Keys that only
/// `encoded` has are fields added since the fixture was written, which older peers skip.
fn keeps_fixture(encoded: &serde_json::Value, fixture: &serde_json::Value) -> bool {
    use serde_json::Value::{Array, Object};
    match (encoded, fixture) {
        (Object(encoded), Object(fixture)) => fixture.iter().all(|(key, fixture)| {
            encoded
                .get(key)
                .is_some_and(|encoded| keeps_fixture(encoded, fixture))
        }),
        (Array(encoded), Array(fixture)) => {
            encoded.len() == fixture.len()
                && encoded
                    .iter()
                    .zip(fixture)
                    .all(|(encoded, fixture)| keeps_fixture(encoded, fixture))
        }
        _ => encoded == fixture,
    }
}

/// Decode every entry of the fixture at `path` as `T`, then encode it again. Peers that speak the
/// fixture's protocol version send these entries, so each must decode. Those peers also read
/// what this side sends, so encoding an entry again must keep everything the fixture has; a
/// renamed or removed field or variant fails here even when decoding still works.
fn check_fixture<T: Serialize + DeserializeOwned>(path: &Path) {
    let fixture: Vec<serde_json::Value> = serde_json::from_str(&read_text(path))
        .unwrap_or_else(|err| panic!("{} is not a JSON array: {err}", path.display()));
    for (index, entry) in fixture.iter().enumerate() {
        let decoded: T = serde_json::from_value(entry.clone()).unwrap_or_else(|err| {
            panic!(
                "{}[{index}] no longer decodes: {err}. Peers that speak this protocol version \
                 still send it, so the change needs a PLUGIN_PROTOCOL_VERSION minor bump.",
                path.display()
            )
        });
        let encoded = serde_json::to_value(&decoded).expect("a decoded entry should encode");
        assert!(
            keeps_fixture(&encoded, entry),
            "{}[{index}] encodes differently now:\n{encoded:#}\nPeers that speak this protocol \
             version read it the old way, so the change needs a PLUGIN_PROTOCOL_VERSION minor \
             bump. If the only difference is a field that is now left out when empty, check \
             that those peers accept it missing.",
            path.display()
        );
    }
}

#[test]
fn frozen_fixtures_decode() -> Result<(), ShellError> {
    let frozen = snapshot_dir().join("frozen");
    assert!(
        frozen.join(PLUGIN_PROTOCOL_VERSION).is_dir(),
        "protocol_snapshots/frozen/{PLUGIN_PROTOCOL_VERSION} is missing; create it with \
         `cargo test -p nu-plugin-protocol -- --ignored regenerate_protocol_snapshots`"
    );
    let current = ProtocolInfo::default();
    for entry in fs::read_dir(&frozen).expect("frozen fixtures exist") {
        let dir = entry.expect("readable frozen fixture directory").path();
        // Skip stray files, such as the `.DS_Store` that macOS Finder leaves behind.
        if !dir.is_dir() {
            continue;
        }
        let version = dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("frozen fixture directories are named by protocol version");
        if current.is_compatible_with(&info(version))? {
            check_fixture::<PluginInput>(&dir.join("plugin_input.json"));
            check_fixture::<PluginOutput>(&dir.join("plugin_output.json"));
            check_fixture::<Variants>(&dir.join("variants.json"));
        }
    }
    Ok(())
}

#[test]
#[ignore = "writes protocol_snapshots; run with --ignored after an intended wire change"]
fn regenerate_protocol_snapshots() {
    // Frozen fixtures are written once per protocol version and never overwritten. Building the
    // corpus before writing anything, and checking each file, lets a run that stopped halfway
    // fill in what is missing. Until a version ships, delete its directory to freeze it again.
    let corpus = corpus();
    let frozen = snapshot_dir().join("frozen").join(PLUGIN_PROTOCOL_VERSION);
    fs::create_dir_all(&frozen).expect("failed to create the frozen fixture directory");
    for (file, text) in corpus {
        fs::write(snapshot_dir().join(file), &text).expect("failed to write snapshot");
        let fixture = frozen.join(file);
        if !fixture.exists() {
            fs::write(fixture, &text).expect("failed to write frozen fixture");
        }
    }
}
