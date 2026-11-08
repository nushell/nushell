use nu_plugin_protocol::PLUGIN_PROTOCOL_VERSION;
use std::path::Path;

/// The example plugins written in other languages, and `nu_plugin_stress_internals`, which
/// writes the protocol by hand, can't import `PLUGIN_PROTOCOL_VERSION`, so they repeat it.
#[test]
fn example_plugins_send_the_current_protocol_version() {
    let version = PLUGIN_PROTOCOL_VERSION;
    let copies = [
        (
            "crates/nu_plugin_python/nu_plugin_python_example.py",
            format!(r#"PLUGIN_PROTOCOL_VERSION = "{version}""#),
        ),
        (
            "crates/nu_plugin_javascript/nu_plugin_node_example.js",
            format!("const PLUGIN_PROTOCOL_VERSION = '{version}';"),
        ),
        (
            "crates/nu_plugin_nu_example/nu_plugin_nu_example.nu",
            format!(r#"const PLUGIN_PROTOCOL_VERSION = "{version}""#),
        ),
        (
            "crates/nu_plugin_stress_internals/src/main.rs",
            format!(r#"const PLUGIN_PROTOCOL_VERSION: &str = "{version}";"#),
        ),
    ];

    for (file, line) in copies {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        assert!(
            source.lines().any(|source_line| source_line == line),
            "{file} should contain `{line}`, matching nu-plugin-protocol's PLUGIN_PROTOCOL_VERSION"
        );
    }
}
