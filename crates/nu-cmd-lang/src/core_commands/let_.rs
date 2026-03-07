use nu_engine::command_prelude::*;
use nu_engine::eval_expression;
use nu_protocol::ast::Expr;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Let;

impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn description(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .required(
                "var_name",
                SyntaxShape::VarWithOptType,
                "The variable name to create.",
            )
            .optional(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by value.",
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
        vec!["set", "const"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let expr = call
            .positional_nth(stack, 0)
            .ok_or(ShellError::NushellFailed {
                msg: "Missing variable name".to_string(),
            })?;
        let var_id = expr.as_var().ok_or(ShellError::NushellFailed {
            msg: "Expected variable".to_string(),
        })?;

        let initial_value = call.get_parser_info(stack, "initial_value").cloned();

        // Evaluate the right-hand side value:
        // - If there's an initial_value (let x = expr), evaluate the expression
        // - Otherwise (let x), use the pipeline input
        let rhs = if let Some(ref initial_value_expr) = initial_value {
            // Validate that blocks/subexpressions don't have multiple pipeline elements
            if let Expr::Block(block_id) | Expr::Subexpression(block_id) = &initial_value_expr.expr
            {
                let block = engine_state.get_block(*block_id);
                if block
                    .pipelines
                    .iter()
                    .any(|pipeline| pipeline.elements.len() > 1)
                {
                    return Err(ShellError::NushellFailed {
                        msg: "invalid `let` keyword call".to_string(),
                    });
                }
            }
            // Discard input when using = syntax
            let _ = input.into_value(call.head)?;
            eval_expression::<WithoutDebug>(engine_state, stack, initial_value_expr)?
        } else {
            // Use pipeline input directly when no = is provided
            input.into_value(call.head)?
        };

        // If the variable is declared `: glob` and the RHS is a string,
        // coerce it to an *expandable* `Value::Glob(..., no_expand = false)` so
        // runtime `let` behavior matches the compiled `GlobFrom { no_expand: false }`.
        // This ensures `let g: glob = "*.toml"; ls $g` expands like a glob
        // literal and keeps parity with `into glob`.
        let value_to_store = {
            let variable = engine_state.get_var(var_id);
            if variable.ty == Type::Glob {
                match &rhs {
                    Value::String { val, .. } => Value::glob(val.clone(), false, rhs.span()),
                    _ => rhs.clone(),
                }
            } else {
                rhs.clone()
            }
        };
        stack.add_var(var_id, value_to_store);

        if initial_value.is_some() {
            // `let var = expr`: suppress output (traditional assignment, no display)
            Ok(PipelineData::Empty)
        } else {
            // `input | let var`: pass through the assigned value
            Ok(PipelineData::Value(rhs, None))
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set a variable to a value (no output).",
                example: "let x = 10",
                result: None,
            },
            Example {
                description: "Set a variable to the result of an expression (no output).",
                example: "let x = 10 + 100",
                result: None,
            },
            Example {
                description: "Set a variable based on the condition (no output).",
                example: "let x = if false { -1 } else { 1 }",
                result: None,
            },
            Example {
                description: "Set a variable to the output of a pipeline.",
                example: "ls | let files",
                result: None,
            },
            Example {
                description: "Use let in the middle of a pipeline to assign and pass the value.",
                example: "10 | let x | $x + 5",
                result: Some(Value::test_int(15)),
            },
            Example {
                description: "Use let in the middle of a pipeline, then consume value with $in.",
                example: "10 | let x | $in + 5",
                result: Some(Value::test_int(15)),
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

        test_examples(Let {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Let.command_type(), CommandType::Keyword));
    }
}
