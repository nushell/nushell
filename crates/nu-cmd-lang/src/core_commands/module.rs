use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Module;

impl Command for Module {
    fn name(&self) -> &str {
        "module"
    }

    fn description(&self) -> &str {
        "Define a custom module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("module")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("module", SyntaxShape::String, "Module name or module path.")
            .optional(
                "block",
                SyntaxShape::Block,
                "Body of the module if 'module' parameter is not a module path.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
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
                description: "Define a custom command in a module and call it",
                example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
                result: Some(Value::test_string("foo")),
            },
            Example {
                description: "Define an environment variable in a module",
                example: r#"module foo { export-env { $env.FOO = "BAZ" } }; use foo; $env.FOO"#,
                result: Some(Value::test_string("BAZ")),
            },
            Example {
                description: "Define a custom command that participates in the environment in a module and call it",
                example: r#"module foo { export def --env bar [] { $env.FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR"#,
                result: Some(Value::test_string("BAZ")),
            },
        ]
    }
}
