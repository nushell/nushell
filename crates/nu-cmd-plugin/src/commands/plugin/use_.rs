use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct PluginUse;

impl Command for PluginUse {
    fn name(&self) -> &str {
        "plugin use"
    }

    fn description(&self) -> &str {
        "Load a plugin from the plugin registry file into scope."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .named(
                "plugin-config",
                SyntaxShape::Filepath,
                "Use a plugin registry file other than the one set in `$nu.plugin-path`",
                None,
            )
            .required(
                "name",
                SyntaxShape::String,
                "The name, or filename, of the plugin to load.",
            )
            .category(Category::Plugin)
    }

    fn extra_description(&self) -> &str {
        r#"
This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html

The plugin definition must be available in the plugin registry file at parse
time. Run `plugin add` first in the REPL to do this, or from a script consider
preparing a plugin registry file and passing `--plugin-config`, or using the
`--plugin` option to `nu` instead.

If the plugin was already loaded, this will reload the latest definition from
the registry file into scope.

Note that even if the plugin filename is specified, it will only be loaded if
it was already previously registered with `plugin add`.
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add", "register", "scope"]
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Load the commands for the `query` plugin from $nu.plugin-path",
                example: r#"plugin use query"#,
                result: None,
            },
            Example {
                description: "Load the commands for the plugin with the filename `~/.cargo/bin/nu_plugin_query` from $nu.plugin-path",
                example: r#"plugin use ~/.cargo/bin/nu_plugin_query"#,
                result: None,
            },
            Example {
                description: "Load the commands for the `query` plugin from a custom plugin registry file",
                example: r#"plugin use --plugin-config local-plugins.msgpackz query"#,
                result: None,
            },
        ]
    }
}
