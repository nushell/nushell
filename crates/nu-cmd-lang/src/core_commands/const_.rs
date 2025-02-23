use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Const;

impl Command for Const {
    fn name(&self) -> &str {
        "const"
    }

    fn description(&self) -> &str {
        "Create a parse-time constant."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("const")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("const_name", SyntaxShape::VarWithOptType, "Constant name.")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by constant value.",
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

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "let"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
        let var_id = if let Some(id) = call.positional_nth(0).and_then(|pos| pos.as_var()) {
            id
        } else {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Could not get variable".to_string(),
                label: "variable not added by the parser".to_string(),
                span: call.head,
            });
        };

        if let Some(constval) = &engine_state.get_var(var_id).const_val {
            stack.add_var(var_id, constval.clone());

            Ok(PipelineData::empty())
        } else {
            Err(ShellError::NushellFailedSpanned {
                msg: "Missing Constant".to_string(),
                label: "constant not added by the parser".to_string(),
                span: call.head,
            })
        }
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }

    fn is_const(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a new parse-time constant.",
                example: "const x = 10",
                result: None,
            },
            Example {
                description: "Create a composite constant value",
                example: "const x = { a: 10, b: 20 }",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use nu_protocol::engine::CommandType;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Const {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Const.command_type(), CommandType::Keyword));
    }
}
