use nu_protocol::{
    ast::Call,
    engine::{EngineState, Stack},
    FromValue, ShellError,
};

use crate::eval_expression;

pub trait CallExt {
    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError>;

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError>;

    fn opt<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError>;

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError>;
}

impl CallExt for Call {
    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.get_flag_expr(name) {
            let result = eval_expression(engine_state, stack, &expr)?;
            FromValue::from_value(&result).map(Some)
        } else {
            Ok(None)
        }
    }

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        for expr in self.positional_iter().skip(starting_pos) {
            let result = eval_expression(engine_state, stack, expr)?;
            output.push(FromValue::from_value(&result)?);
        }

        Ok(output)
    }

    fn opt<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.positional_nth(pos) {
            let result = eval_expression(engine_state, stack, expr)?;
            FromValue::from_value(&result).map(Some)
        } else {
            Ok(None)
        }
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        if let Some(expr) = self.positional_nth(pos) {
            let result = eval_expression(engine_state, stack, expr)?;
            FromValue::from_value(&result)
        } else if self.positional_len() == 0 {
            Err(ShellError::AccessEmptyContent(self.head))
        } else {
            Err(ShellError::AccessBeyondEnd(
                self.positional_len() - 1,
                self.head,
            ))
        }
    }
}
