use itertools::{EitherOrBoth, Itertools};
use nu_engine::command_prelude::*;
use nu_plugin_engine::{GetPlugin, PersistentPlugin};
use nu_protocol::{
    IntoValue, PluginGcConfig, PluginIdentity, PluginMetadata, PluginRegistryItemData,
};
use std::sync::Arc;

use crate::util::read_plugin_file;

#[derive(Clone)]
pub struct PluginList;

impl Command for PluginList {
    fn name(&self) -> &str {
        "plugin list"
    }

    fn signature(&self) -> Signature {
        Signature::build("plugin list")
            .input_output_type(
                Type::Nothing,
                Type::Table(
                    vec![
                        ("name".into(), Type::String),
                        ("version".into(), Type::String),
                        ("protocol_version".into(), Type::String),
                        ("nushell_version".into(), Type::String),
                        ("status".into(), Type::String),
                        ("pid".into(), Type::Int),
                        ("filename".into(), Type::String),
                        ("shell".into(), Type::String),
                        (
                            "commands".into(),
                            Type::List(Box::new(Type::Record(
                                vec![
                                    ("name".into(), Type::String),
                                    ("description".into(), Type::String),
                                ]
                                .into(),
                            ))),
                        ),
                    ]
                    .into(),
                ),
            )
            .named(
                "plugin-config",
                SyntaxShape::Filepath,
                "Use a plugin registry file other than the one set in `$nu.plugin-path`.",
                None,
            )
            .switch(
                "engine",
                "Show info for plugins that are loaded into the engine only.",
                Some('e'),
            )
            .switch(
                "registry",
                "Show info for plugins from the registry file only.",
                Some('r'),
            )
            .category(Category::Plugin)
    }

    fn description(&self) -> &str {
        "List loaded and installed plugins."
    }

    fn extra_description(&self) -> &str {
        "
The `status` column will contain one of the following values:

- `added`:    The plugin is present in the plugin registry file, but not in
              the engine.
- `loaded`:   The plugin is present both in the plugin registry file and in
              the engine, but is not running.
- `running`:  The plugin is currently running, and the `pid` column should
              contain its process ID.
- `modified`: The plugin state present in the plugin registry file is different
              from the state in the engine.
- `removed`:  The plugin is still loaded in the engine, but is not present in
              the plugin registry file.
- `invalid`:  The data in the plugin registry file couldn't be deserialized,
              and the plugin most likely needs to be added again.

`running` takes priority over any other status. Unless `--registry` is used
or the plugin has not been loaded yet, the values of `version`, `filename`,
`shell`, and `commands` reflect the values in the engine and not the ones in
the plugin registry file.

See also: `plugin use`
"
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["scope"]
    }

    fn examples(&self) -> Vec<nu_protocol::Example<'_>> {
        vec![
            Example {
                example: "plugin list",
                description: "List installed plugins.",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::test_string("inc"),
                    "version" => Value::test_string(env!("CARGO_PKG_VERSION")),
                    "protocol_version" => Value::test_string("0.93.0"),
                    "nushell_version" => Value::test_string(env!("CARGO_PKG_VERSION")),
                    "status" => Value::test_string("running"),
                    "pid" => Value::test_int(106480),
                    "filename" => if cfg!(windows) {
                        Value::test_string(r"C:\nu\plugins\nu_plugin_inc.exe")
                    } else {
                        Value::test_string("/opt/nu/plugins/nu_plugin_inc")
                    },
                    "shell" => Value::test_nothing(),
                    "commands" => Value::test_list(vec![Value::test_record(record! {
                        "name" => Value::test_string("inc"),
                        "description" => Value::test_string("Increment a number by 1."),
                    })]),
                })])),
            },
            Example {
                example: "ps | where pid in (plugin list).pid",
                description: "Get process information for running plugins.",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let custom_path = call.get_flag(engine_state, stack, "plugin-config")?;
        let engine_mode = call.has_flag(engine_state, stack, "engine")?;
        let registry_mode = call.has_flag(engine_state, stack, "registry")?;

        let plugins_info = match (engine_mode, registry_mode) {
            // --engine and --registry together is equivalent to the default.
            (false, false) | (true, true) => {
                if engine_state.plugin_path.is_some() || custom_path.is_some() {
                    let plugins_in_engine = get_plugins_in_engine(engine_state, stack);
                    let plugins_in_registry =
                        get_plugins_in_registry(engine_state, stack, call.head, &custom_path)?;
                    merge_plugin_info(plugins_in_engine, plugins_in_registry)
                } else {
                    // Don't produce error when running nu --no-config-file
                    get_plugins_in_engine(engine_state, stack)
                }
            }
            (true, false) => get_plugins_in_engine(engine_state, stack),
            (false, true) => get_plugins_in_registry(engine_state, stack, call.head, &custom_path)?,
        };

        Ok(plugins_info.into_value(call.head).into_pipeline_data())
    }
}

#[derive(Debug, Clone, IntoValue, PartialOrd, Ord, PartialEq, Eq)]
struct PluginInfo {
    name: String,
    version: Option<String>,
    protocol_version: Option<String>,
    nushell_version: Option<String>,
    status: PluginStatus,
    pid: Option<u32>,
    filename: String,
    shell: Option<String>,
    commands: Vec<CommandInfo>,
}

#[derive(Debug, Clone, IntoValue, PartialOrd, Ord, PartialEq, Eq)]
struct CommandInfo {
    name: String,
    description: String,
}

#[derive(Debug, Clone, Copy, IntoValue, PartialOrd, Ord, PartialEq, Eq)]
#[nu_value(rename_all = "snake_case")]
enum PluginStatus {
    Added,
    Loaded,
    Running,
    Modified,
    Removed,
    Invalid,
}

// Builds plugin info for plugins currently loaded in the engine.
fn get_plugins_in_engine(engine_state: &EngineState, stack: &mut Stack) -> Vec<PluginInfo> {
    // Group plugin decls by plugin identity
    let decls = engine_state.plugin_decls().into_group_map_by(|decl| {
        decl.plugin_identity()
            .expect("plugin decl should have identity")
    });

    // Build plugins list
    engine_state
        .plugins()
        .iter()
        .map(|plugin| {
            let cached_metadata = plugin.metadata();
            let needs_live_metadata = cached_metadata
                .as_ref()
                .map(|m| {
                    m.version.is_none()
                        || m.protocol_version.is_none()
                        || m.nushell_version.is_none()
                })
                .unwrap_or(true);
            let live_metadata = if needs_live_metadata {
                fetch_current_plugin_metadata(engine_state, stack, plugin.identity())
            } else {
                None
            };

            let (version, protocol_version, nushell_version) =
                metadata_fields_with_fallback(cached_metadata.as_ref(), live_metadata.as_ref());

            // Find commands that belong to the plugin
            let commands: Vec<(String, String)> = decls
                .get(plugin.identity())
                .into_iter()
                .flat_map(|decls| {
                    decls
                        .iter()
                        .map(|decl| (decl.name().to_owned(), decl.description().to_owned()))
                })
                .sorted()
                .collect();

            PluginInfo {
                name: plugin.identity().name().into(),
                version,
                protocol_version,
                nushell_version,
                status: if plugin.pid().is_some() {
                    PluginStatus::Running
                } else {
                    PluginStatus::Loaded
                },
                pid: plugin.pid(),
                filename: plugin.identity().filename().to_string_lossy().into_owned(),
                shell: plugin
                    .identity()
                    .shell()
                    .map(|path| path.to_string_lossy().into_owned()),
                commands: commands
                    .iter()
                    .map(|(name, desc)| CommandInfo {
                        name: name.clone(),
                        description: desc.clone(),
                    })
                    .collect(),
            }
        })
        .sorted()
        .collect()
}

// Merges cached metadata with live metadata, preferring cached fields when present.
fn metadata_fields_with_fallback(
    cached: Option<&PluginMetadata>,
    live: Option<&PluginMetadata>,
) -> (Option<String>, Option<String>, Option<String>) {
    let mut version = cached.and_then(|m| m.version.clone());
    let mut protocol_version = cached.and_then(|m| m.protocol_version.clone());
    let mut nushell_version = cached.and_then(|m| m.nushell_version.clone());

    if let Some(live) = live {
        if version.is_none() {
            version = live.version.clone();
        }
        if protocol_version.is_none() {
            protocol_version = live.protocol_version.clone();
        }
        if nushell_version.is_none() {
            nushell_version = live.nushell_version.clone();
        }
    }

    (version, protocol_version, nushell_version)
}

// Starts a temporary plugin instance and asks it for current metadata.
fn fetch_current_plugin_metadata(
    engine_state: &EngineState,
    stack: &mut Stack,
    identity: &PluginIdentity,
) -> Option<PluginMetadata> {
    let plugin = Arc::new(PersistentPlugin::new(
        identity.clone(),
        PluginGcConfig {
            enabled: true,
            stop_after: 0,
        },
    ));

    plugin
        .get_plugin(Some((engine_state, stack)))
        .ok()?
        .get_metadata()
        .ok()
}

// Reads plugin entries from the registry file and maps them to display rows.
fn get_plugins_in_registry(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
    custom_path: &Option<Spanned<String>>,
) -> Result<Vec<PluginInfo>, ShellError> {
    let plugin_file_contents = read_plugin_file(engine_state, stack, span, custom_path)?;

    let plugins_info = plugin_file_contents
        .plugins
        .into_iter()
        .map(|plugin| {
            let mut info = PluginInfo {
                name: plugin.name,
                version: None,
                protocol_version: None,
                nushell_version: None,
                status: PluginStatus::Added,
                pid: None,
                filename: plugin.filename.to_string_lossy().into_owned(),
                shell: plugin.shell.map(|path| path.to_string_lossy().into_owned()),
                commands: vec![],
            };

            if let PluginRegistryItemData::Valid { metadata, commands } = plugin.data {
                info.version = metadata.version;
                info.protocol_version = metadata.protocol_version;
                info.nushell_version = metadata.nushell_version;
                info.commands = commands
                    .into_iter()
                    .map(|command| CommandInfo {
                        name: command.sig.name.clone(),
                        description: command.sig.description.clone(),
                    })
                    .sorted()
                    .collect();
            } else {
                info.status = PluginStatus::Invalid;
            }
            info
        })
        .sorted()
        .collect();

    Ok(plugins_info)
}

/// If no options are provided, the command loads from both the plugin list in the engine and what's
/// in the registry file. We need to reconcile the two to set the proper states and make sure that
/// new plugins that were added to the plugin registry file show up.
///
/// Engine fields remain authoritative, while metadata may be backfilled from registry/live data.
fn merge_plugin_info(
    from_engine: Vec<PluginInfo>,
    from_registry: Vec<PluginInfo>,
) -> Vec<PluginInfo> {
    from_engine
        .into_iter()
        .merge_join_by(from_registry, |info_a, info_b| {
            info_a.name.cmp(&info_b.name)
        })
        .map(|either_or_both| match either_or_both {
            // Exists in the engine, but not in the registry file
            EitherOrBoth::Left(info) => PluginInfo {
                status: match info.status {
                    PluginStatus::Running => info.status,
                    // The plugin is not in the registry file, so it should be marked as `removed`
                    _ => PluginStatus::Removed,
                },
                ..info
            },
            // Exists in the registry file, but not in the engine
            EitherOrBoth::Right(info) => info,
            // Exists in both
            EitherOrBoth::Both(info_engine, info_registry) => {
                let info_engine = info_engine.with_metadata_fallback_from(&info_registry);
                let info_registry = info_registry.with_metadata_fallback_from(&info_engine);

                PluginInfo {
                    status: match (info_engine.status, info_registry.status) {
                        // Above all, `running` should be displayed if the plugin is running
                        (PluginStatus::Running, _) => PluginStatus::Running,
                        // `invalid` takes precedence over other states because the user probably wants
                        // to fix it
                        (_, PluginStatus::Invalid) => PluginStatus::Invalid,
                        // Display `modified` if the state in the registry is different somehow
                        _ if info_engine.is_modified(&info_registry) => PluginStatus::Modified,
                        // Otherwise, `loaded` (it's not running)
                        _ => PluginStatus::Loaded,
                    },
                    ..info_engine
                }
            }
        })
        .sorted()
        .collect()
}

impl PluginInfo {
    // Fills missing metadata fields from another PluginInfo while preserving existing values.
    fn with_metadata_fallback_from(mut self, other: &PluginInfo) -> Self {
        self.version = self.version.or_else(|| other.version.clone());
        self.protocol_version = self
            .protocol_version
            .or_else(|| other.protocol_version.clone());
        self.nushell_version = self
            .nushell_version
            .or_else(|| other.nushell_version.clone());
        self
    }

    /// True if the plugin info shows some kind of change (other than status/pid) relative to the
    /// other
    fn is_modified(&self, other: &PluginInfo) -> bool {
        self.name != other.name
            || self.filename != other.filename
            || self.shell != other.shell
            || self.commands != other.commands
            || self.version != other.version
            || self.protocol_version != other.protocol_version
            || self.nushell_version != other.nushell_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::PluginMetadata;

    // Creates minimal plugin info for targeted merge behavior tests.
    fn plugin_info(name: &str) -> PluginInfo {
        PluginInfo {
            name: name.into(),
            version: None,
            protocol_version: None,
            nushell_version: None,
            status: PluginStatus::Loaded,
            pid: None,
            filename: format!("/plugins/nu_plugin_{name}"),
            shell: None,
            commands: vec![],
        }
    }

    // Ensures registry metadata fills missing engine metadata in merged output.
    #[test]
    fn merge_plugin_info_uses_registry_metadata_when_engine_metadata_is_missing() {
        let engine = plugin_info("gstat");
        let registry = PluginInfo {
            version: Some("0.112.3".into()),
            protocol_version: Some("0.93.0".into()),
            nushell_version: Some("0.112.3".into()),
            ..plugin_info("gstat")
        };

        let merged = merge_plugin_info(vec![engine], vec![registry]);
        let merged = merged
            .into_iter()
            .next()
            .expect("expected merged plugin info");

        assert_eq!(Some("0.112.3"), merged.version.as_deref());
        assert_eq!(Some("0.93.0"), merged.protocol_version.as_deref());
        assert_eq!(Some("0.112.3"), merged.nushell_version.as_deref());
        assert_eq!(PluginStatus::Loaded, merged.status);
    }

    // Ensures engine fields stay authoritative even when metadata is backfilled from registry.
    #[test]
    fn merge_plugin_info_keeps_engine_fields_while_filling_metadata_from_registry() {
        let engine = PluginInfo {
            filename: "/engine/nu_plugin_gstat".into(),
            commands: vec![CommandInfo {
                name: "gstat".into(),
                description: "engine command".into(),
            }],
            ..plugin_info("gstat")
        };
        let registry = PluginInfo {
            filename: "/registry/nu_plugin_gstat".into(),
            protocol_version: Some("0.93.0".into()),
            nushell_version: Some("0.112.3".into()),
            ..plugin_info("gstat")
        };

        let merged = merge_plugin_info(vec![engine], vec![registry]);
        let merged = merged
            .into_iter()
            .next()
            .expect("expected merged plugin info");

        assert_eq!("/engine/nu_plugin_gstat", merged.filename);
        assert_eq!(1, merged.commands.len());
        assert_eq!(Some("0.93.0"), merged.protocol_version.as_deref());
        assert_eq!(Some("0.112.3"), merged.nushell_version.as_deref());
        assert_eq!(PluginStatus::Modified, merged.status);
    }

    // Ensures live metadata fills protocol/nushell fields when cache only has version.
    #[test]
    fn metadata_fields_with_fallback_fills_missing_fields_from_live_metadata() {
        let cached = PluginMetadata::new().with_version("0.111.1");
        let live = PluginMetadata::new()
            .with_version("0.112.3")
            .with_protocol_version("0.93.0")
            .with_nushell_version("0.112.3");

        let (version, protocol_version, nushell_version) =
            metadata_fields_with_fallback(Some(&cached), Some(&live));

        assert_eq!(Some("0.111.1"), version.as_deref());
        assert_eq!(Some("0.93.0"), protocol_version.as_deref());
        assert_eq!(Some("0.112.3"), nushell_version.as_deref());
    }

    // Ensures missing fields remain empty when no live metadata is available.
    #[test]
    fn metadata_fields_with_fallback_keeps_missing_fields_when_live_metadata_unavailable() {
        let cached = PluginMetadata::new().with_version("0.111.1");

        let (version, protocol_version, nushell_version) =
            metadata_fields_with_fallback(Some(&cached), None);

        assert_eq!(Some("0.111.1"), version.as_deref());
        assert_eq!(None, protocol_version.as_deref());
        assert_eq!(None, nushell_version.as_deref());
    }
}
