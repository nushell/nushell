use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Var;

impl Command for Var {
    fn name(&self) -> &str {
        "var"
    }

    fn usage(&self) -> &str {
        "Create a variable from a pipeline."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("var")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("var_name", SyntaxShape::VarWithOptType, "Variable name.")
            .category(Category::Core)
    }

    fn is_parser_keyword(&self) -> bool {
        true // this is not true but needs to be, we should probably just build this into let
    }

    fn extra_usage(&self) -> &str {
        "This should work like `let` but gets the variable value from the end of the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "const", "let"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_span = input.span().unwrap_or(Span::unknown());
        let var_id = call
            .positional_nth(0)
            .expect("already checked positional")
            .as_var()
            .expect("internal error: missing variable");

        let input_as_value = input.into_value(input_span);

        stack.add_var(var_id, input_as_value);
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set a variable to a value",
                example: "echo 10 | var val;echo $val",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Set a variable to the result of an expression",
                example: "10 + 100 | var expr;echo $expr",
                result: Some(Value::test_int(110)),
            },
            Example {
                description: "Set a variable based on the condition",
                example: "if false { -1 } else { 1 } | var cond;echo $cond",
                result: Some(Value::test_int(1)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Var {})
    }
}
