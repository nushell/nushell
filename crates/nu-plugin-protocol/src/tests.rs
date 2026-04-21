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
fn protocol_info_default_is_compatible_with_same_protocol_major() -> Result<(), ShellError> {
    let current = ProtocolInfo::default();
    let compatible = ProtocolInfo {
        protocol: Protocol::NuPlugin,
        version: "0.93.1".into(),
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

#[test]
fn protocol_schema_matches_snapshot() {
    let actual = crate::schema::plugin_protocol_schema_json()
        .expect("plugin protocol schema should serialize to json");
    let expected = include_str!("../protocol_schema/plugin_protocol.schema.json");

    assert_json_snapshot(actual, expected);
}
