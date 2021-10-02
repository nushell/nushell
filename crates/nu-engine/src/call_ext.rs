use nu_protocol::{ast::Call, engine::EvaluationContext, ShellError};

use crate::{eval_expression, FromValue};

pub trait CallExt {
    fn get_flag<T: FromValue>(
        &self,
        context: &EvaluationContext,
        name: &str,
    ) -> Result<Option<T>, ShellError>;

    fn rest<T: FromValue>(
        &self,
        context: &EvaluationContext,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError>;
}

impl CallExt for Call {
    fn get_flag<T: FromValue>(
        &self,
        context: &EvaluationContext,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.get_flag_expr(name) {
            let result = eval_expression(context, &expr)?;
            FromValue::from_value(&result).map(Some)
        } else {
            Ok(None)
        }
    }

    fn rest<T: FromValue>(
        &self,
        context: &EvaluationContext,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        for expr in self.positional.iter().skip(starting_pos) {
            let result = eval_expression(context, expr)?;
            output.push(FromValue::from_value(&result)?);
        }

        Ok(output)
    }
}
