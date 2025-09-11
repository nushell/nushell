use nu_engine::command_prelude::*;

use crate::util::{canonicalize_possible_filename_arg, modify_plugin_file};

#[derive(Clone)]
pub struct PluginRm;

impl Command for PluginRm {
    fn name(&self) -> &str {
        "plugin rm"
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
            .switch(
                "force",
                "Don't cause an error if the plugin name wasn't found in the file",
                Some('f'),
            )
            .required(
                "name",
                SyntaxShape::String,
                "The name, or filename, of the plugin to remove.",
            )
            .category(Category::Plugin)
    }

    fn description(&self) -> &str {
        "Remove a plugin from the plugin registry file."
    }

    fn extra_description(&self) -> &str {
        r#"
This does not remove the plugin commands from the current scope or from `plugin
list` in the current shell. It instead removes the plugin from the plugin
registry file (by default, `$nu.plugin-path`). The changes will be apparent the
next time `nu` is launched with that plugin registry file.

This can be useful for removing an invalid plugin signature, if it can't be
fixed with `plugin add`.
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["remove", "delete", "signature"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "plugin rm inc",
                description: "Remove the installed signatures for the `inc` plugin.",
                result: None,
            },
            Example {
                example: "plugin rm ~/.cargo/bin/nu_plugin_inc",
                description: "Remove the installed signatures for the plugin with the filename `~/.cargo/bin/nu_plugin_inc`.",
                result: None,
            },
            Example {
                example: "plugin rm --plugin-config polars.msgpackz polars",
                description: "Remove the installed signatures for the `polars` plugin from the \"polars.msgpackz\" plugin registry file.",
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
        let name: Spanned<String> = call.req(engine_state, stack, 0)?;
        let custom_path = call.get_flag(engine_state, stack, "plugin-config")?;
        let force = call.has_flag(engine_state, stack, "force")?;

        let filename = canonicalize_possible_filename_arg(engine_state, stack, &name.item);

        modify_plugin_file(engine_state, stack, call.head, &custom_path, |contents| {
            if let Some(index) = contents
                .plugins
                .iter()
                .position(|p| p.name == name.item || p.filename == filename)
            {
                contents.plugins.remove(index);
                Ok(())
            } else if force {
                Ok(())
            } else {
                Err(ShellError::GenericError {
                    error: format!("Failed to remove the `{}` plugin", name.item),
                    msg: "couldn't find a plugin with this name in the registry file".into(),
                    span: Some(name.span),
                    help: None,
                    inner: vec![],
                })
            }
        })?;

        Ok(Value::nothing(call.head).into_pipeline_data())
    }
}
