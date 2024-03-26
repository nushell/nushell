use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ExportDef;

impl Command for ExportDef {
    fn name(&self) -> &str {
        "export def"
    }

    fn usage(&self) -> &str {
        "Define a custom command and export it from a module."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "Command name.")
            .required("params", SyntaxShape::Signature, "Parameters.")
            .required("block", SyntaxShape::Block, "Body of the definition.")
            .switch("env", "keep the environment defined inside the command", None)
            .switch("wrapped", "treat unknown flags and arguments as strings (requires ...rest-like parameter in signature)", None)
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
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Define a custom command in a module and call it",
            example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
            result: Some(Value::test_string("foo")),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
