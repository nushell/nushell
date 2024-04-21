use nu_engine::command_prelude::*;

use crate::util::modify_plugin_file;

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
                "Use a plugin cache file other than the one set in `$nu.plugin-path`",
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
                "The name of the plugin to remove (not the filename)",
            )
            .category(Category::Plugin)
    }

    fn usage(&self) -> &str {
        "Remove a plugin from the plugin cache file."
    }

    fn extra_usage(&self) -> &str {
        r#"
This does not remove the plugin commands from the current scope or from `plugin
list` in the current shell. It instead removes the plugin from the plugin
cache file (by default, `$nu.plugin-path`). The changes will be apparent the
next time `nu` is launched with that plugin cache file.

This can be useful for removing an invalid plugin signature, if it can't be
fixed with `plugin add`.
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["plugin", "rm", "remove", "delete", "signature"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "plugin rm inc",
                description: "Remove the installed signatures for the `inc` plugin.",
                result: None,
            },
            Example {
                example: "plugin rm --plugin-config polars.msgpackz polars",
                description: "Remove the installed signatures for the `polars` plugin from the \"polars.msgpackz\" plugin cache file.",
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

        modify_plugin_file(engine_state, stack, call.head, custom_path, |contents| {
            if !force && !contents.plugins.iter().any(|p| p.name == name.item) {
                Err(ShellError::GenericError {
                    error: format!("Failed to remove the `{}` plugin", name.item),
                    msg: "couldn't find a plugin with this name in the cache file".into(),
                    span: Some(name.span),
                    help: None,
                    inner: vec![],
                })
            } else {
                contents.remove_plugin(&name.item);
                Ok(())
            }
        })?;

        Ok(Value::nothing(call.head).into_pipeline_data())
    }
}
