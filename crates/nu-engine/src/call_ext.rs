use nu_protocol::{
    ast::{Call, Expression},
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::eval_constant,
    FromValue, ShellError, Value,
};

use crate::eval_expression;

pub trait CallExt {
    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError>;

    fn get_flag_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        name: &str,
    ) -> Result<Option<T>, ShellError>;

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError>;

    fn rest_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError>;

    fn rest_iter_flattened<F>(&self, start: usize, eval: F) -> Result<Vec<Value>, ShellError>
    where
        F: FnMut(&Expression) -> Result<Value, ShellError>;

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

    fn req_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<T, ShellError>;

    fn req_parser_info<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
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
            let result = eval_expression(engine_state, stack, expr)?;
            FromValue::from_value(result).map(Some)
        } else {
            Ok(None)
        }
    }

    fn get_flag_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.get_flag_expr(name) {
            let result = eval_constant(working_set, expr)?;
            FromValue::from_value(result).map(Some)
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

        for result in self.rest_iter_flattened(starting_pos, |expr| {
            eval_expression(engine_state, stack, expr)
        })? {
            output.push(FromValue::from_value(result)?);
        }

        Ok(output)
    }

    fn rest_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        for result in
            self.rest_iter_flattened(starting_pos, |expr| eval_constant(working_set, expr))?
        {
            output.push(FromValue::from_value(result)?);
        }

        Ok(output)
    }

    fn rest_iter_flattened<F>(&self, start: usize, mut eval: F) -> Result<Vec<Value>, ShellError>
    where
        F: FnMut(&Expression) -> Result<Value, ShellError>,
    {
        let mut output = Vec::new();

        for (expr, spread) in self.rest_iter(start) {
            let result = eval(expr)?;
            if spread {
                match result {
                    Value::List { mut vals, .. } => output.append(&mut vals),
                    _ => return Err(ShellError::CannotSpreadAsList { span: expr.span }),
                }
            } else {
                output.push(result);
            }
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
            FromValue::from_value(result).map(Some)
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
            FromValue::from_value(result)
        } else if self.positional_len() == 0 {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional_len() - 1,
                span: self.head,
            })
        }
    }

    fn req_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<T, ShellError> {
        if let Some(expr) = self.positional_nth(pos) {
            let result = eval_constant(working_set, expr)?;
            FromValue::from_value(result)
        } else if self.positional_len() == 0 {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional_len() - 1,
                span: self.head,
            })
        }
    }

    fn req_parser_info<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<T, ShellError> {
        if let Some(expr) = self.get_parser_info(name) {
            let result = eval_expression(engine_state, stack, expr)?;
            FromValue::from_value(result)
        } else if self.parser_info.is_empty() {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.parser_info.len() - 1,
                span: self.head,
            })
        }
    }
}
