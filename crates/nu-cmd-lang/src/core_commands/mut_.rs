use nu_engine::{command_prelude::*, get_eval_block};

#[derive(Clone)]
pub struct Mut;

impl Command for Mut {
    fn name(&self) -> &str {
        "mut"
    }

    fn usage(&self) -> &str {
        "Create a mutable variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("mut")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("var_name", SyntaxShape::VarWithOptType, "Variable name.")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by value.",
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

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "mutable"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let var_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_var()
            .expect("internal error: missing variable");

        let block_id = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing right hand side");

        let block = engine_state.get_block(block_id);
        let eval_block = get_eval_block(engine_state);
        let stack = &mut stack.start_capture();
        let pipeline_data = eval_block(engine_state, stack, block, input)?;
        let value = pipeline_data.into_value(call.head);

        // if given variable type is Glob, and our result is string
        // then nushell need to convert from Value::String to Value::Glob
        // it's assigned by demand, then it's not quoted, and it's required to expand
        // if we pass it to other commands.
        let var_type = &engine_state.get_var(var_id).ty;
        let val_span = value.span();
        let value = match value {
            Value::String { val, .. } if var_type == &Type::Glob => {
                Value::glob(val, false, val_span)
            }
            value => value,
        };

        stack.add_var(var_id, value);
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set a mutable variable to a value, then update it",
                example: "mut x = 10; $x = 12",
                result: None,
            },
            Example {
                description: "Upsert a value inside a mutable data structure",
                example: "mut a = {b:{c:1}}; $a.b.c = 2",
                result: None,
            },
            Example {
                description: "Set a mutable variable to the result of an expression",
                example: "mut x = 10 + 100",
                result: None,
            },
            Example {
                description: "Set a mutable variable based on the condition",
                example: "mut x = if false { -1 } else { 1 }",
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

        test_examples(Mut {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Mut.command_type(), CommandType::Keyword));
    }
}
