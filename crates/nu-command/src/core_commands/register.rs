use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Register;

impl Command for Register {
    fn name(&self) -> &str {
        "register"
    }

    fn usage(&self) -> &str {
        "Register a plugin"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("register")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "plugin",
                SyntaxShape::Filepath,
                "path of executable for plugin",
            )
            .optional(
                "signature",
                SyntaxShape::Any,
                "Block with signature description as json object",
            )
            .named(
                "shell",
                SyntaxShape::Filepath,
                "path of shell used to run plugin (cmd, sh, python, etc)",
                Some('s'),
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Register `nu_plugin_query` plugin from ~/.cargo/bin/ dir",
                example: r#"register ~/.cargo/bin/nu_plugin_query"#,
                result: None,
            },
            Example {
                description: "Register `nu_plugin_query` plugin from `nu -c`(plugin will be available in that nu session only)",
                example: r#"let plugin = ((which nu).path.0 | path dirname | path join 'nu_plugin_query'); nu -c $'register ($plugin); version'"#,
                result: None,
            },
        ]
    }
}
