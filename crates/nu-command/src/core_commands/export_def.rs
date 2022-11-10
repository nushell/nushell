use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct ExportDef;

impl Command for ExportDef {
    fn name(&self) -> &str {
        "export def"
    }

    fn usage(&self) -> &str {
        "Define a custom command and export it from a module"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("export def")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("block", SyntaxShape::Block, "body of the definition")
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
        vec![Example {
            description: "Define a custom command in a module and call it",
            example: r#"module spam { export def foo [] { "foo" } }; use spam foo; foo"#,
            result: Some(Value::String {
                val: "foo".to_string(),
                span: Span::test_data(),
            }),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["module"]
    }
}
