use nu_engine::command_prelude::*;
use nu_protocol::engine::{ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID};

#[derive(Clone)]
pub struct DeleteVar;

impl Command for DeleteVar {
    fn name(&self) -> &str {
        "unlet"
    }

    fn description(&self) -> &str {
        "Delete variables from nushell memory, making them unrecoverable."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("unlet")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "rest",
                SyntaxShape::Any,
                "The variables to delete (pass as $variable_name).",
            )
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Collect all positional arguments passed to the command
        let expressions: Vec<_> = (0..).map_while(|i| call.positional_nth(stack, i)).collect();

        // Ensure at least one argument is provided
        if expressions.is_empty() {
            return Err(ShellError::GenericError {
                error: "Wrong number of arguments".into(),
                msg: "unlet takes at least one argument".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }

        // Validate each argument and collect valid variable IDs
        let mut var_ids = Vec::with_capacity(expressions.len());
        for expr in expressions {
            match &expr.expr {
                nu_protocol::ast::Expr::Var(var_id) => {
                    // Prevent deletion of built-in variables that are essential for nushell operation
                    if var_id == &NU_VARIABLE_ID
                        || var_id == &ENV_VARIABLE_ID
                        || var_id == &IN_VARIABLE_ID
                    {
                        // Determine the variable name for the error message
                        let var_name = match *var_id {
                            NU_VARIABLE_ID => "nu",
                            ENV_VARIABLE_ID => "env",
                            IN_VARIABLE_ID => "in",
                            _ => "unknown", // This should never happen due to the check above
                        };

                        return Err(ShellError::GenericError {
                            error: "Cannot delete built-in variable".into(),
                            msg: format!(
                                "'${}' is a built-in variable and cannot be deleted",
                                var_name
                            ),
                            span: Some(expr.span),
                            help: None,
                            inner: vec![],
                        });
                    }
                    var_ids.push(*var_id);
                }
                _ => {
                    // Argument is not a variable reference
                    return Err(ShellError::GenericError {
                        error: "Not a variable".into(),
                        msg: "Argument must be a variable reference like $x".into(),
                        span: Some(expr.span),
                        help: Some("Use $variable_name to refer to the variable".into()),
                        inner: vec![],
                    });
                }
            }
        }

        // Remove all valid variables from the stack
        for var_id in var_ids {
            stack.remove_var(var_id);
        }

        Ok(PipelineData::empty())
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "let x = 42; unlet $x",
                description: "Delete a variable from memory.",
                result: None,
            },
            Example {
                example: "let x = 1; let y = 2; unlet $x $y",
                description: "Delete multiple variables from memory.",
                result: None,
            },
            Example {
                example: "unlet $nu",
                description: "Attempting to delete a built-in variable fails.",
                result: None,
            },
            Example {
                example: "unlet 42",
                description: "Attempting to delete a non-variable fails.",
                result: None,
            },
        ]
    }
}
