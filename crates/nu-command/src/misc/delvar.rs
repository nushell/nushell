use nu_engine::command_prelude::*;
use nu_protocol::engine::{ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID};

#[derive(Clone)]
pub struct DeleteVar;

impl Command for DeleteVar {
    fn name(&self) -> &str {
        "delvar"
    }

    fn description(&self) -> &str {
        "Delete a variable from nushell memory, making it unrecoverable."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("delvar")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "variable",
                SyntaxShape::Any,
                "The variable to delete (pass as $variable_name).",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Get the single required positional argument
        let Some(expr) = call.positional_nth(stack, 0) else {
            return Err(ShellError::GenericError {
                error: "Wrong number of arguments".into(),
                msg: "delvar takes exactly one argument".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        };

        // Check if the expression is a variable reference
        match &expr.expr {
            nu_protocol::ast::Expr::Var(var_id) => {
                // Prevent deletion of built-in variables
                if var_id == &NU_VARIABLE_ID
                    || var_id == &ENV_VARIABLE_ID
                    || var_id == &IN_VARIABLE_ID
                {
                    return Err(ShellError::GenericError {
                        error: "Cannot delete built-in variable".into(),
                        msg: format!(
                            "'${}' is a built-in variable and cannot be deleted",
                            match var_id {
                                var_id if var_id == &NU_VARIABLE_ID => "nu",
                                var_id if var_id == &ENV_VARIABLE_ID => "env",
                                var_id if var_id == &IN_VARIABLE_ID => "in",
                                _ => unreachable!(),
                            }
                        ),
                        span: Some(expr.span),
                        help: None,
                        inner: vec![],
                    });
                }
                // Remove the variable from the stack
                stack.remove_var(*var_id);
                Ok(PipelineData::empty())
            }
            _ => Err(ShellError::GenericError {
                error: "Not a variable".into(),
                msg: "Argument must be a variable reference like $x".into(),
                span: Some(expr.span),
                help: Some("Use $variable_name to refer to the variable".into()),
                inner: vec![],
            }),
        }
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }
}

impl DeleteVar {}
