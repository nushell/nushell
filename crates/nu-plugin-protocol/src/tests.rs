use super::*;

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
