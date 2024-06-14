use crate::eval_expression;
use nu_protocol::{
    ast::Call,
    debugger::WithoutDebug,
    engine::{self, CallImpl, EngineState, Stack, StateWorkingSet},
    eval_const::eval_constant,
    ir, FromValue, ShellError, Value,
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

impl CallExt for ir::Call<'_> {
    fn has_flag(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        Ok(self
            .named_iter()
            .find(|(name, _)| *name == flag_name)
            .is_some())
    }

    fn get_flag<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(val) = self.get_named_arg(name) {
            T::from_value(val.clone()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn rest<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        self.rest_iter_flattened(starting_pos)?
            .into_iter()
            .map(T::from_value)
            .collect()
    }

    fn opt<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        self.positional_iter()
            .nth(pos)
            .cloned()
            .map(T::from_value)
            .transpose()
    }

    fn opt_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        todo!("opt_const is not yet implemented on ir::Call")
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        if let Some(val) = self.opt(engine_state, stack, pos)? {
            Ok(val)
        } else if self.positional_len() == 0 {
            Err(ShellError::AccessEmptyContent { span: *self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional_len() - 1,
                span: *self.head,
            })
        }
    }

    fn req_parser_info<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<T, ShellError> {
        todo!("req_parser_info is not yet implemented on ir::Call")
    }
}

impl CallExt for engine::Call<'_> {
    fn has_flag(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.has_flag(engine_state, stack, flag_name),
            CallImpl::Ir(ir_call) => ir_call.has_flag(engine_state, stack, flag_name),
        }
    }

    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.get_flag(engine_state, stack, name),
            CallImpl::Ir(ir_call) => ir_call.get_flag(engine_state, stack, name),
        }
    }

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.rest(engine_state, stack, starting_pos),
            CallImpl::Ir(ir_call) => ir_call.rest(engine_state, stack, starting_pos),
        }
    }

    fn opt<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.opt(engine_state, stack, pos),
            CallImpl::Ir(ir_call) => ir_call.opt(engine_state, stack, pos),
        }
    }

    fn opt_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.opt_const(working_set, pos),
            CallImpl::Ir(ir_call) => ir_call.opt_const(working_set, pos),
        }
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.req(engine_state, stack, pos),
            CallImpl::Ir(ir_call) => ir_call.req(engine_state, stack, pos),
        }
    }

    fn req_parser_info<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<T, ShellError> {
        match &self.inner {
            CallImpl::Ast(ast_call) => ast_call.req_parser_info(engine_state, stack, name),
            CallImpl::Ir(ir_call) => ir_call.req_parser_info(engine_state, stack, name),
        }
    }
}
