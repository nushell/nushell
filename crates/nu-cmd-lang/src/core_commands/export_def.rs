use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportDef;

impl Command for ExportDef {
    fn name(&self) -> &str {
        "export def"
    }

    fn description(&self) -> &str {
        "Define a custom command and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "Command name to define.")
            .required("params", SyntaxShape::Signature, "Command parameters: comma-separated list inside [].")
            .required("block", SyntaxShape::Block, "Command body: list of instructions inside {}.")
            .switch("env", "Environment: defined inside the command.", None)
            .switch("wrapped", "Unknown flags and arguments: strings that require rest-like parameter in signature.", None)
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
        vec![Example {
            description: "Define a custom command in a module and call it.",
            example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
            result: Some(Value::test_string("foo")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
