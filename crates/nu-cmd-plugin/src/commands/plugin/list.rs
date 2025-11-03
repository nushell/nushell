use itertools::{EitherOrBoth, Itertools};
use nu_engine::command_prelude::*;
use nu_protocol::{IntoValue, PluginRegistryItemData};

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
                    [
                        ("name".into(), Type::String),
                        ("version".into(), Type::String),
                        ("status".into(), Type::String),
                        ("pid".into(), Type::Int),
                        ("filename".into(), Type::String),
                        ("shell".into(), Type::String),
                        ("commands".into(), Type::List(Type::String.into())),
                    ]
                    .into(),
                ),
            )
            .named(
                "plugin-config",
                SyntaxShape::Filepath,
                "Use a plugin registry file other than the one set in `$nu.plugin-path`",
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
        r#"
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
"#
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
                    "status" => Value::test_string("running"),
                    "pid" => Value::test_int(106480),
                    "filename" => if cfg!(windows) {
                        Value::test_string(r"C:\nu\plugins\nu_plugin_inc.exe")
                    } else {
                        Value::test_string("/opt/nu/plugins/nu_plugin_inc")
                    },
                    "shell" => Value::test_nothing(),
                    "commands" => Value::test_list(vec![Value::test_string("inc")]),
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
                    let plugins_in_engine = get_plugins_in_engine(engine_state);
                    let plugins_in_registry =
                        get_plugins_in_registry(engine_state, stack, call.head, &custom_path)?;
                    merge_plugin_info(plugins_in_engine, plugins_in_registry)
                } else {
                    // Don't produce error when running nu --no-config-file
                    get_plugins_in_engine(engine_state)
                }
            }
            (true, false) => get_plugins_in_engine(engine_state),
            (false, true) => get_plugins_in_registry(engine_state, stack, call.head, &custom_path)?,
        };

        Ok(plugins_info.into_value(call.head).into_pipeline_data())
    }
}

#[derive(Debug, Clone, IntoValue, PartialOrd, Ord, PartialEq, Eq)]
struct PluginInfo {
    name: String,
    version: Option<String>,
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

fn get_plugins_in_engine(engine_state: &EngineState) -> Vec<PluginInfo> {
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
                version: plugin.metadata().and_then(|m| m.version),
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
                status: PluginStatus::Added,
                pid: None,
                filename: plugin.filename.to_string_lossy().into_owned(),
                shell: plugin.shell.map(|path| path.to_string_lossy().into_owned()),
                commands: vec![],
            };

            if let PluginRegistryItemData::Valid { metadata, commands } = plugin.data {
                info.version = metadata.version;
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
            EitherOrBoth::Both(info_engine, info_registry) => PluginInfo {
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
            },
        })
        .sorted()
        .collect()
}

impl PluginInfo {
    /// True if the plugin info shows some kind of change (other than status/pid) relative to the
    /// other
    fn is_modified(&self, other: &PluginInfo) -> bool {
        self.name != other.name
            || self.filename != other.filename
            || self.shell != other.shell
            || self.commands != other.commands
    }
}
