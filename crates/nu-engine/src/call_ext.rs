use crate::eval_expression;
use nu_protocol::{
    ast,
    debugger::WithoutDebug,
    engine::{self, EngineState, Stack, StateWorkingSet},
    eval_const::eval_constant,
    ir, FromValue, ShellError, Span, Value,
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

    /// Efficiently get the span of a flag argument
    fn get_flag_span(&self, stack: &Stack, name: &str) -> Option<Span>;

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

    /// True if the command has any positional or rest arguments, excluding before the given index.
    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool;
}

impl CallExt for ast::Call {
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

    fn get_flag_span(&self, _stack: &Stack, name: &str) -> Option<Span> {
        self.get_named_arg(name).map(|arg| arg.span)
    }

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        let stack = &mut stack.use_call_arg_out_dest();
        self.rest_iter_flattened(starting_pos, |expr| {
            eval_expression::<WithoutDebug>(engine_state, stack, expr)
        })?
        .into_iter()
        .map(FromValue::from_value)
        .collect()
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

    fn has_positional_args(&self, _stack: &Stack, starting_pos: usize) -> bool {
        self.rest_iter(starting_pos).next().is_some()
    }
}

impl CallExt for ir::Call {
    fn has_flag(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        Ok(self
            .named_iter(stack)
            .find(|(name, _)| name.item == flag_name)
            .is_some_and(|(_, value)| {
                // Handle --flag=false
                !matches!(value, Some(Value::Bool { val: false, .. }))
            }))
    }

    fn get_flag<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        if let Some(val) = self.get_named_arg(stack, name) {
            T::from_value(val.clone()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn get_flag_span(&self, stack: &Stack, name: &str) -> Option<Span> {
        self.named_iter(stack)
            .find_map(|(i_name, _)| (i_name.item == name).then_some(i_name.span))
    }

    fn rest<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        self.rest_iter_flattened(stack, starting_pos)?
            .into_iter()
            .map(T::from_value)
            .collect()
    }

    fn opt<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        self.positional_iter(stack)
            .nth(pos)
            .cloned()
            .map(T::from_value)
            .transpose()
    }

    fn opt_const<T: FromValue>(
        &self,
        _working_set: &StateWorkingSet,
        _pos: usize,
    ) -> Result<Option<T>, ShellError> {
        Err(ShellError::IrEvalError {
            msg: "const evaluation is not yet implemented on ir::Call".into(),
            span: Some(self.head),
        })
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        if let Some(val) = self.opt(engine_state, stack, pos)? {
            Ok(val)
        } else if self.positional_len(stack) == 0 {
            Err(ShellError::AccessEmptyContent { span: self.head })
        } else {
            Err(ShellError::AccessBeyondEnd {
                max_idx: self.positional_len(stack) - 1,
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
        // FIXME: this depends on the AST evaluator. We can fix this by making the parser info an
        // enum rather than using expressions. It's not clear that evaluation of this is ever really
        // needed.
        if let Some(expr) = self.get_parser_info(stack, name) {
            let expr = expr.clone();
            let stack = &mut stack.use_call_arg_out_dest();
            let result = eval_expression::<WithoutDebug>(engine_state, stack, &expr)?;
            FromValue::from_value(result)
        } else {
            Err(ShellError::CantFindColumn {
                col_name: name.into(),
                span: None,
                src_span: self.head,
            })
        }
    }

    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool {
        self.rest_iter(stack, starting_pos).next().is_some()
    }
}

macro_rules! proxy {
    ($self:ident . $method:ident ($($param:expr),*)) => (match &$self.inner {
        engine::CallImpl::AstRef(call) => call.$method($($param),*),
        engine::CallImpl::AstBox(call) => call.$method($($param),*),
        engine::CallImpl::IrRef(call) => call.$method($($param),*),
        engine::CallImpl::IrBox(call) => call.$method($($param),*),
    })
}

impl CallExt for engine::Call<'_> {
    fn has_flag(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        proxy!(self.has_flag(engine_state, stack, flag_name))
    }

    fn get_flag<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        proxy!(self.get_flag(engine_state, stack, name))
    }

    fn get_flag_span(&self, stack: &Stack, name: &str) -> Option<Span> {
        proxy!(self.get_flag_span(stack, name))
    }

    fn rest<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        proxy!(self.rest(engine_state, stack, starting_pos))
    }

    fn opt<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        proxy!(self.opt(engine_state, stack, pos))
    }

    fn opt_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        proxy!(self.opt_const(working_set, pos))
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        proxy!(self.req(engine_state, stack, pos))
    }

    fn req_parser_info<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<T, ShellError> {
        proxy!(self.req_parser_info(engine_state, stack, name))
    }

    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool {
        proxy!(self.has_positional_args(stack, starting_pos))
    }
}
