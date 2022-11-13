use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct ExportDefEnv;

impl Command for ExportDefEnv {
    fn name(&self) -> &str {
        "export def-env"
    }

    fn usage(&self) -> &str {
        "Define a custom command that participates in the environment and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def-env")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("block", SyntaxShape::Block, "body of the definition")
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html

=== EXTRA NOTE ===
All blocks are scoped, including variable definition and environment variable changes.

Because of this, the following doesn't work:

export def-env cd_with_fallback [arg = ""] {
    let fall_back_path = "/tmp"
    if $arg != "" {
        cd $arg
    } else {
        cd $fall_back_path
    }
}

Instead, you have to use cd in the top level scope:

export def-env cd_with_fallback [arg = ""] {
    let fall_back_path = "/tmp"
    let path = if $arg != "" {
        $arg
    } else {
        $fall_back_path
    }
    cd $path
}"#
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
        vec![Example {
            description: "Define a custom command that participates in the environment in a module and call it",
            example: r#"module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
            result: Some(Value::String {
                val: "BAZ".to_string(),
                span: Span::test_data(),
            }),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
