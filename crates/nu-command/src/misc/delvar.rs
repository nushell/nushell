use nu_engine::command_prelude::*;
use nu_protocol::engine::{ENV_VARIABLE_ID, IN_VARIABLE_ID, NU_VARIABLE_ID};
use std::sync::Arc;

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
        // Extract positional arguments from the call, handling both AST and IR representations
        let positional = Self::extract_positional_args(call, stack);

        if positional.len() != 1 {
            return Err(ShellError::GenericError {
                error: "Wrong number of arguments".into(),
                msg: "delvar takes exactly one argument".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }

        let expr = positional[0].as_ref();

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

impl DeleteVar {
    /// Helper function to extract positional arguments from the call.
    /// This handles the different representations of calls (AST and IR).
    fn extract_positional_args(
        call: &Call,
        stack: &mut Stack,
    ) -> Vec<Arc<nu_protocol::ast::Expression>> {
        match &call.inner {
            nu_protocol::engine::CallImpl::AstRef(ast_call) => ast_call
                .positional_iter()
                .map(|e| Arc::new(e.clone()))
                .collect::<Vec<_>>(),
            nu_protocol::engine::CallImpl::AstBox(ast_call) => ast_call
                .positional_iter()
                .map(|e| Arc::new(e.clone()))
                .collect::<Vec<_>>(),
            nu_protocol::engine::CallImpl::IrRef(ir_call) => {
                let arguments = ir_call.arguments(stack);
                arguments
                    .iter()
                    .filter_map(|arg| match arg {
                        nu_protocol::engine::Argument::Positional {
                            ast: Some(expr), ..
                        } => Some(expr.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            }
            nu_protocol::engine::CallImpl::IrBox(ir_call) => {
                let arguments = ir_call.arguments(stack);
                arguments
                    .iter()
                    .filter_map(|arg| match arg {
                        nu_protocol::engine::Argument::Positional {
                            ast: Some(expr), ..
                        } => Some(expr.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            }
        }
    }
}
