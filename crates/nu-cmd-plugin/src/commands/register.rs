use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Register;

impl Command for Register {
    fn name(&self) -> &str {
        "register"
    }

    fn usage(&self) -> &str {
        "Register a plugin."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("register")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "plugin",
                SyntaxShape::Filepath,
                "Path of executable for plugin.",
            )
            .optional(
                "signature",
                SyntaxShape::Any,
                "Block with signature description as json object.",
            )
            .named(
                "shell",
                SyntaxShape::Filepath,
                "path of shell used to run plugin (cmd, sh, python, etc)",
                Some('s'),
            )
            .category(Category::Plugin)
    }

    fn extra_usage(&self) -> &str {
        r#"
Deprecated in favor of `plugin add` and `plugin use`.

This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html
"#
        .trim()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add"]
    }

    fn is_parser_keyword(&self) -> bool {
        true
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Register `nu_plugin_query` plugin from ~/.cargo/bin/ dir",
                example: r#"register ~/.cargo/bin/nu_plugin_query"#,
                result: None,
            },
            Example {
                description: "Register `nu_plugin_query` plugin from `nu -c` (writes/updates $nu.plugin-path)",
                example: r#"let plugin = ((which nu).path.0 | path dirname | path join 'nu_plugin_query'); nu -c $'register ($plugin); version'"#,
                result: None,
            },
        ]
    }
}
