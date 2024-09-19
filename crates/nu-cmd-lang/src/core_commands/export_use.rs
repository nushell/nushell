use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct ExportUse;

impl Command for ExportUse {
    fn name(&self) -> &str {
        "export use"
    }

    fn description(&self) -> &str {
        "Use definitions from a module and export them from this module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export use")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("module", SyntaxShape::String, "Module or module file.")
            .rest(
                "members",
                SyntaxShape::Any,
                "Which members of the module to import.",
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

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Re-export a command from another module",
            example: r#"module spam { export def foo [] { "foo" } }
    module eggs { export use spam foo }
    use eggs foo
    foo
            "#,
            result: Some(Value::test_string("foo")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["reexport", "import", "module"]
    }
}
