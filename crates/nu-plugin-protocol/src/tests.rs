use super::*;
use serde_json::json;

fn assert_json_snapshot(actual: serde_json::Value, snapshot: &str) {
    let expected: serde_json::Value =
        serde_json::from_str(snapshot).expect("snapshot must be valid json");
    assert_eq!(actual, expected);
}

fn sample_plugin_input_run() -> PluginInput {
    PluginInput::Call(
        1,
        PluginCall::Run(CallInfo {
            name: "test".into(),
            call: EvaluatedCall {
                head: Span::new(0, 10),
                positional: vec![
                    Value::float(1.0, Span::new(11, 12)),
                    Value::string("something", Span::new(13, 22)),
                ],
                named: vec![(
                    Spanned {
                        item: "flag".into(),
                        span: Span::new(23, 27),
                    },
                    Some(Value::int(9, Span::new(28, 29))),
                )],
            },
            input: PipelineDataHeader::Value(Value::bool(false, Span::new(30, 35)), None),
        }),
    )
}

fn sample_plugin_output_engine_call() -> PluginOutput {
    PluginOutput::EngineCall {
        context: 3,
        id: 4,
        call: EngineCall::GetEnvVar("PWD".into()),
    }
}

#[test]
fn protocol_info_compatible() -> Result<(), ShellError> {
    let ver_1_2_3 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.2.3".into(),
        features: vec![],
    };
    let ver_1_1_0 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.1.0".into(),
        features: vec![],
    };
    assert!(ver_1_1_0.is_compatible_with(&ver_1_2_3)?);
    assert!(ver_1_2_3.is_compatible_with(&ver_1_1_0)?);
    Ok(())
}

#[test]
fn protocol_info_incompatible() -> Result<(), ShellError> {
    let ver_2_0_0 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "2.0.0".into(),
        features: vec![],
    };
    let ver_1_1_0 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.1.0".into(),
        features: vec![],
    };
    assert!(!ver_2_0_0.is_compatible_with(&ver_1_1_0)?);
    assert!(!ver_1_1_0.is_compatible_with(&ver_2_0_0)?);
    Ok(())
}

#[test]
fn protocol_info_compatible_with_nightly() -> Result<(), ShellError> {
    let ver_1_2_3 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.2.3".into(),
        features: vec![],
    };
    let ver_1_1_0 = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.1.0-nightly.1".into(),
        features: vec![],
    };
    assert!(ver_1_1_0.is_compatible_with(&ver_1_2_3)?);
    assert!(ver_1_2_3.is_compatible_with(&ver_1_1_0)?);
    Ok(())
}

#[test]
fn protocol_info_default_uses_protocol_version_constant() {
    assert_eq!(ProtocolInfo::default().version, PLUGIN_PROTOCOL_VERSION);
}

#[test]
fn protocol_info_default_is_compatible_with_same_protocol_minor() -> Result<(), ShellError> {
    let current = ProtocolInfo::default();
    let compatible = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.1.1".into(),
        features: vec![],
    };

    assert!(current.is_compatible_with(&compatible)?);
    assert!(compatible.is_compatible_with(&current)?);
    Ok(())
}

#[test]
fn protocol_info_default_rejects_incompatible_major() -> Result<(), ShellError> {
    let current = ProtocolInfo::default();
    let incompatible = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "1.0.0".into(),
        features: vec![],
    };

    assert!(!current.is_compatible_with(&incompatible)?);
    assert!(!incompatible.is_compatible_with(&current)?);
    Ok(())
}

#[test]
fn protocol_info_json_shape_is_explicit() {
    let expected_features = if cfg!(feature = "local-socket") {
        json!([{ "name": "LocalSocket" }])
    } else {
        json!([])
    };

    let actual =
        serde_json::to_value(ProtocolInfo::default()).expect("protocol info should serialize");

    assert_eq!(
        actual,
        json!({
            "protocol": "nu-plugin",
            "version": PLUGIN_PROTOCOL_VERSION,
            "features": expected_features,
        })
    );
}

#[test]
fn plugin_input_run_json_shape_is_explicit() {
    let input = sample_plugin_input_run();

    let actual = serde_json::to_value(&input).expect("plugin input should serialize");

    assert_eq!(
        actual,
        json!({
            "Call": [
                1,
                {
                    "Run": {
                        "name": "test",
                        "call": {
                            "head": { "start": 0, "end": 10 },
                            "positional": [
                                { "Float": { "val": 1.0, "span": { "start": 11, "end": 12 } } },
                                { "String": { "val": "something", "span": { "start": 13, "end": 22 } } }
                            ],
                            "named": [
                                [
                                    { "item": "flag", "span": { "start": 23, "end": 27 } },
                                    { "Int": { "val": 9, "span": { "start": 28, "end": 29 } } }
                                ]
                            ]
                        },
                        "input": {
                            "Value": [
                                { "Bool": { "val": false, "span": { "start": 30, "end": 35 } } },
                                null
                            ]
                        }
                    }
                }
            ]
        })
    );
}

#[test]
fn plugin_output_engine_call_json_shape_is_explicit() {
    let output = sample_plugin_output_engine_call();

    let actual = serde_json::to_value(&output).expect("plugin output should serialize");

    assert_eq!(
        actual,
        json!({
            "EngineCall": {
                "context": 3,
                "id": 4,
                "call": { "GetEnvVar": "PWD" }
            }
        })
    );
}

#[test]
fn plugin_custom_value_false_notify_is_omitted() {
    let value = PluginCustomValue::new("Foo".into(), vec![1, 2, 3], false);

    let actual = serde_json::to_value(&value).expect("custom value should serialize");

    assert_eq!(actual, json!({ "name": "Foo", "data": [1, 2, 3] }));
}

#[test]
fn unknown_feature_cannot_serialize() {
    let err = serde_json::to_value(Feature::Unknown).expect_err("unknown feature should fail");

    assert!(err.to_string().contains("unknown protocol feature"));
}

#[test]
fn protocol_info_matches_snapshot() {
    let actual =
        serde_json::to_value(ProtocolInfo::default()).expect("protocol info should serialize");
    let expected = include_str!("../protocol_snapshots/protocol_info_default.json");

    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_input_run_matches_snapshot() {
    let input = sample_plugin_input_run();

    let actual = serde_json::to_value(&input).expect("plugin input should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_input_run.json");

    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_output_engine_call_matches_snapshot() {
    let output = sample_plugin_output_engine_call();

    let actual = serde_json::to_value(&output).expect("plugin output should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_output_engine_call.json");

    assert_json_snapshot(actual, expected);
}

#[cfg(feature = "schema")]
#[test]
fn protocol_schema_matches_snapshot() {
    let actual = crate::schema::plugin_protocol_schema_json()
        .expect("plugin protocol schema should serialize to json");
    let expected = include_str!("../protocol_schema/plugin_protocol.schema.json");

    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_input_signal_json_shape_is_explicit() {
    let input = PluginInput::Signal(nu_protocol::SignalAction::Interrupt);
    let actual = serde_json::to_value(&input).expect("signal should serialize");
    assert_eq!(actual, json!({ "Signal": "Interrupt" }));
}

#[test]
fn plugin_output_hello_roundtrips() {
    let output = PluginOutput::Hello(ProtocolInfo::default());
    let json = serde_json::to_value(&output).expect("hello should serialize");
    let back: PluginOutput = serde_json::from_value(json).expect("hello should deserialize");
    match back {
        PluginOutput::Hello(info) => {
            assert_eq!(info.version, PLUGIN_PROTOCOL_VERSION);
            assert!(matches!(info.protocol, Protocol::NuPlugin));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn plugin_call_response_metadata_includes_protocol_fields() {
    use nu_protocol::PluginMetadata;
    let meta = PluginMetadata::new()
        .with_version("1.2.3")
        .with_protocol_version(PLUGIN_PROTOCOL_VERSION)
        .with_nushell_version("0.114.2");
    let response = PluginCallResponse::<PipelineDataHeader>::Metadata(meta);
    let actual = serde_json::to_value(&response).expect("metadata response should serialize");
    assert_eq!(
        actual,
        json!({
            "Metadata": {
                "version": "1.2.3",
                "protocol_version": PLUGIN_PROTOCOL_VERSION,
                "nushell_version": "0.114.2",
            }
        })
    );
}

fn sample_plugin_call_signature() -> PluginInput {
    PluginInput::Call(2, PluginCall::Signature)
}

fn sample_plugin_call_get_completion() -> PluginInput {
    use nu_protocol::ast::Call;
    PluginInput::Call(
        3,
        PluginCall::GetCompletion(GetCompletionInfo {
            name: "demo".into(),
            arg_type: GetCompletionArgType::Flag("path".into()),
            call: DynamicCompletionCall {
                call: Call::new(Span::new(0, 4)),
                strip: true,
                pos: 4,
            },
        }),
    )
}

fn sample_plugin_call_custom_value_op() -> PluginInput {
    PluginInput::Call(
        4,
        PluginCall::CustomValueOp(
            Spanned {
                item: PluginCustomValue::new("MyType".into(), vec![1, 2, 3], false),
                span: Span::new(5, 12),
            },
            CustomValueOp::ToBaseValue,
        ),
    )
}

fn sample_plugin_call_response_signature() -> PluginOutput {
    use nu_protocol::PluginSignature;
    PluginOutput::CallResponse(
        5,
        PluginCallResponse::Signature(vec![PluginSignature::build("demo")]),
    )
}

#[test]
fn plugin_input_signature_matches_snapshot() {
    let actual = serde_json::to_value(sample_plugin_call_signature())
        .expect("signature call should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_input_signature.json");
    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_input_get_completion_matches_snapshot() {
    let actual = serde_json::to_value(sample_plugin_call_get_completion())
        .expect("get completion should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_input_get_completion.json");
    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_input_custom_value_op_matches_snapshot() {
    let actual = serde_json::to_value(sample_plugin_call_custom_value_op())
        .expect("custom value op should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_input_custom_value_op.json");
    assert_json_snapshot(actual, expected);
}

#[test]
fn plugin_output_signature_response_matches_snapshot() {
    let actual = serde_json::to_value(sample_plugin_call_response_signature())
        .expect("signature response should serialize");
    let expected = include_str!("../protocol_snapshots/plugin_output_signature_response.json");
    assert_json_snapshot(actual, expected);
}

#[test]
#[ignore = "run with --ignored to regenerate protocol_snapshots JSON fixtures"]
fn dump_new_protocol_snapshots() {
    use std::path::PathBuf;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protocol_snapshots");
    let fixtures = [
        (
            "plugin_input_signature.json",
            serde_json::to_value(sample_plugin_call_signature()).unwrap(),
        ),
        (
            "plugin_input_get_completion.json",
            serde_json::to_value(sample_plugin_call_get_completion()).unwrap(),
        ),
        (
            "plugin_input_custom_value_op.json",
            serde_json::to_value(sample_plugin_call_custom_value_op()).unwrap(),
        ),
        (
            "plugin_output_signature_response.json",
            serde_json::to_value(sample_plugin_call_response_signature()).unwrap(),
        ),
    ];
    for (name, value) in fixtures {
        let pretty = serde_json::to_string_pretty(&value).unwrap() + "\n";
        std::fs::write(dir.join(name), pretty).unwrap();
        println!("wrote {}", dir.join(name).display());
    }
}
