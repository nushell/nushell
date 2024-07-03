use crate::eval_expression;
use nu_protocol::{
    ast::Call,
    debugger::WithoutDebug,
    engine::{EngineState, Stack, StateWorkingSet},
    eval_const::eval_constant,
    FromValue, ShellError, Value,
};

pub trait CallExt {
    /// Check if a boolean flag is set (i.e. `--bool` or `--bool=true`)
    fn has_flag(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError>;

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

    fn opt_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<Option<T>, ShellError>;

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
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
    fn has_flag(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        for name in self.named_iter() {
            if flag_name == name.0.item {
                return if let Some(expr) = &name.2 {
                    // Check --flag=false
                    let stack = &mut stack.use_call_arg_out_dest();
                    let result = eval_expression::<WithoutDebug>(engine_state, stack, expr)?;
                    match result {
                        Value::Bool { val, .. } => Ok(val),
                        _ => Err(ShellError::CantConvert {
                            to_type: "bool".into(),
                            from_type: result.get_type().to_string(),
                            span: result.span(),
                            help: Some("".into()),
                        }),
                    }
                } else {
                    Ok(true)
                };
            }
        }

        Ok(false)
    }

    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.get_flag_expr(name) {
            let stack = &mut stack.use_call_arg_out_dest();
            let result = eval_expression::<WithoutDebug>(engine_state, stack, expr)?;
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
        let stack = &mut stack.use_call_arg_out_dest();
        let mut output = vec![];

        for result in self.rest_iter_flattened(starting_pos, |expr| {
            eval_expression::<WithoutDebug>(engine_state, stack, expr)
        })? {
            output.push(FromValue::from_value(result)?);
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
            let stack = &mut stack.use_call_arg_out_dest();
            let result = eval_expression::<WithoutDebug>(engine_state, stack, expr)?;
            FromValue::from_value(result).map(Some)
        } else {
            Ok(None)
        }
    }

    fn opt_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.positional_nth(pos) {
            let result = eval_constant(working_set, expr)?;
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
            let stack = &mut stack.use_call_arg_out_dest();
            let result = eval_expression::<WithoutDebug>(engine_state, stack, expr)?;
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
            let stack = &mut stack.use_call_arg_out_dest();
            let result = eval_expression::<WithoutDebug>(engine_state, stack, expr)?;
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
