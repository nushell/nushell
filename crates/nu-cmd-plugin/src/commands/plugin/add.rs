#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_plugin_engine::{GetPlugin, PersistentPlugin};
use nu_protocol::{PluginGcConfig, PluginIdentity, PluginRegistryItem, RegisteredPlugin};
use std::sync::Arc;

use crate::util::{get_plugin_dirs, modify_plugin_file};

#[derive(Clone)]
pub struct PluginAdd;

impl Command for PluginAdd {
    fn name(&self) -> &str {
        "plugin add"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Nothing, Type::Nothing)
            // This matches the option to `nu`
            .named(
                "plugin-config",
                SyntaxShape::Filepath,
                "Use a plugin registry file other than the one set in `$nu.plugin-path`",
                None,
            )
            .named(
                "shell",
                SyntaxShape::Filepath,
                "Use an additional shell program (cmd, sh, python, etc.) to run the plugin",
                Some('s'),
            )
            .required(
                "filename",
                SyntaxShape::Filepath,
                "Path to the executable for the plugin",
            )
            .category(Category::Plugin)
    }

    fn usage(&self) -> &str {
        "Add a plugin to the plugin registry file."
    }

    fn extra_usage(&self) -> &str {
        r#"
This does not load the plugin commands into the scope - see `register` for that.

Instead, it runs the plugin to get its command signatures, and then edits the
plugin registry file (by default, `$nu.plugin-path`). The changes will be
apparent the next time `nu` is next launched with that plugin registry file.
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["load", "register", "signature"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "plugin add nu_plugin_inc",
                description: "Run the `nu_plugin_inc` plugin from the current directory or $env.NU_PLUGIN_DIRS and install its signatures.",
                result: None,
            },
            Example {
                example: "plugin add --plugin-config polars.msgpackz nu_plugin_polars",
                description: "Run the `nu_plugin_polars` plugin from the current directory or $env.NU_PLUGIN_DIRS, and install its signatures to the \"polars.msgpackz\" plugin registry file.",
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
        let filename: Spanned<String> = call.req(engine_state, stack, 0)?;
        let shell: Option<Spanned<String>> = call.get_flag(engine_state, stack, "shell")?;

        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;

        // Check the current directory, or fall back to NU_PLUGIN_DIRS
        let filename_expanded = nu_path::locate_in_dirs(&filename.item, &cwd, || {
            get_plugin_dirs(engine_state, stack)
        })
        .err_span(filename.span)?;

        let shell_expanded = shell
            .as_ref()
            .map(|s| nu_path::canonicalize_with(&s.item, &cwd).err_span(s.span))
            .transpose()?;

        // Parse the plugin filename so it can be used to spawn the plugin
        let identity = PluginIdentity::new(filename_expanded, shell_expanded).map_err(|_| {
            ShellError::GenericError {
                error: "Plugin filename is invalid".into(),
                msg: "plugin executable files must start with `nu_plugin_`".into(),
                span: Some(filename.span),
                help: None,
                inner: vec![],
            }
        })?;

        let custom_path = call.get_flag(engine_state, stack, "plugin-config")?;

        // Start the plugin manually, to get the freshest signatures and to not affect engine
        // state. Provide a GC config that will stop it ASAP
        let plugin = Arc::new(PersistentPlugin::new(
            identity,
            PluginGcConfig {
                enabled: true,
                stop_after: 0,
            },
        ));
        let interface = plugin.clone().get_plugin(Some((engine_state, stack)))?;
        let commands = interface.get_signature()?;

        modify_plugin_file(engine_state, stack, call.head, custom_path, |contents| {
            // Update the file with the received signatures
            let item = PluginRegistryItem::new(plugin.identity(), commands);
            contents.upsert_plugin(item);
            Ok(())
        })?;

        Ok(Value::nothing(call.head).into_pipeline_data())
    }
}
