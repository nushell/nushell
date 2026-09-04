use crate::{
    DeclId, FromValue, ShellError, Span, Value,
    ast::{self, Expression},
    ir,
};

use super::{EngineState, Stack, StateWorkingSet};

/// This is a HACK to help [`Command`](super::Command) support both the old AST evaluator and the
/// new IR evaluator at the same time. It should be removed once we are satisfied with the new
/// evaluator.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub head: Span,
    pub decl_id: DeclId,
    pub inner: CallImpl<'a>,
}

#[derive(Debug, Clone)]
pub enum CallImpl<'a> {
    AstRef(&'a ast::Call),
    AstBox(Box<ast::Call>),
    IrRef(&'a ir::Call),
    IrBox(Box<ir::Call>),
}

impl Call<'_> {
    /// Returns a new AST call with the given span. This is often used by commands that need an
    /// empty call to pass to a command. It's not easily possible to add anything to this.
    pub fn new(span: Span) -> Self {
        // this is using the boxed variant, which isn't so efficient... but this is only temporary
        // anyway.
        Call {
            head: span,
            decl_id: DeclId::new(0),
            inner: CallImpl::AstBox(Box::new(ast::Call::new(span))),
        }
    }

    /// Convert the `Call` from any lifetime into `'static`, by cloning the data within onto the
    /// heap.
    pub fn to_owned(&self) -> Call<'static> {
        Call {
            head: self.head,
            decl_id: self.decl_id,
            inner: self.inner.to_owned(),
        }
    }

    /// Check if a boolean flag is set at const-eval time.
    pub fn has_flag_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        match &self.inner {
            CallImpl::AstRef(call) => call.has_flag_const(working_set, flag_name),
            CallImpl::AstBox(call) => call.has_flag_const(working_set, flag_name),
            CallImpl::IrRef(call) => ir_has_flag_const(call, stack, flag_name),
            CallImpl::IrBox(call) => ir_has_flag_const(call, stack, flag_name),
        }
    }

    /// Get a typed named argument at const-eval time.
    pub fn get_flag_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        match &self.inner {
            CallImpl::AstRef(call) => call.get_flag_const(working_set, name),
            CallImpl::AstBox(call) => call.get_flag_const(working_set, name),
            CallImpl::IrRef(call) => ir_get_flag_const(call, stack, name),
            CallImpl::IrBox(call) => ir_get_flag_const(call, stack, name),
        }
    }

    /// Get a required positional argument at const-eval time.
    pub fn req_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        match &self.inner {
            CallImpl::AstRef(call) => call.req_const(working_set, pos),
            CallImpl::AstBox(call) => call.req_const(working_set, pos),
            CallImpl::IrRef(call) => ir_req_const(call, stack, self.head, pos),
            CallImpl::IrBox(call) => ir_req_const(call, stack, self.head, pos),
        }
    }

    /// Get the rest of the positional arguments at const-eval time.
    pub fn rest_const<T: FromValue>(
        &self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        match &self.inner {
            CallImpl::AstRef(call) => call.rest_const(working_set, starting_pos),
            CallImpl::AstBox(call) => call.rest_const(working_set, starting_pos),
            CallImpl::IrRef(call) => ir_rest_const(call, stack, starting_pos),
            CallImpl::IrBox(call) => ir_rest_const(call, stack, starting_pos),
        }
    }

    /// Returns a span covering the call's arguments.
    pub fn arguments_span(&self) -> Span {
        match &self.inner {
            CallImpl::AstRef(call) => call.arguments_span(),
            CallImpl::AstBox(call) => call.arguments_span(),
            CallImpl::IrRef(call) => call.arguments_span(),
            CallImpl::IrBox(call) => call.arguments_span(),
        }
    }

    /// Returns a span covering the whole call.
    pub fn span(&self) -> Span {
        match &self.inner {
            CallImpl::AstRef(call) => call.span(),
            CallImpl::AstBox(call) => call.span(),
            CallImpl::IrRef(call) => call.span(),
            CallImpl::IrBox(call) => call.span(),
        }
    }

    /// Get a parser info argument by name.
    pub fn get_parser_info<'a>(&'a self, stack: &'a Stack, name: &str) -> Option<&'a Expression> {
        match &self.inner {
            CallImpl::AstRef(call) => call.get_parser_info(name),
            CallImpl::AstBox(call) => call.get_parser_info(name),
            CallImpl::IrRef(call) => call.get_parser_info(stack, name),
            CallImpl::IrBox(call) => call.get_parser_info(stack, name),
        }
    }

    /// Evaluator-agnostic implementation of `rest_iter_flattened()`. Evaluates or gets all of the
    /// positional and spread arguments, flattens spreads, and then returns one list of values.
    pub fn rest_iter_flattened(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        eval_expression: fn(
            &EngineState,
            &mut Stack,
            &ast::Expression,
        ) -> Result<Value, ShellError>,
        starting_pos: usize,
    ) -> Result<Vec<Value>, ShellError> {
        fn by_ast(
            call: &ast::Call,
            engine_state: &EngineState,
            stack: &mut Stack,
            eval_expression: fn(
                &EngineState,
                &mut Stack,
                &ast::Expression,
            ) -> Result<Value, ShellError>,
            starting_pos: usize,
        ) -> Result<Vec<Value>, ShellError> {
            call.rest_iter_flattened(starting_pos, |expr| {
                eval_expression(engine_state, stack, expr)
            })
        }

        fn by_ir(
            call: &ir::Call,
            stack: &Stack,
            starting_pos: usize,
        ) -> Result<Vec<Value>, ShellError> {
            call.rest_iter_flattened(stack, starting_pos)
        }

        match &self.inner {
            CallImpl::AstRef(call) => {
                by_ast(call, engine_state, stack, eval_expression, starting_pos)
            }
            CallImpl::AstBox(call) => {
                by_ast(call, engine_state, stack, eval_expression, starting_pos)
            }
            CallImpl::IrRef(call) => by_ir(call, stack, starting_pos),
            CallImpl::IrBox(call) => by_ir(call, stack, starting_pos),
        }
    }

    /// Get the original AST expression for a positional argument. Does not usually work for IR
    /// unless the decl specified `requires_ast_for_arguments()`
    pub fn positional_nth<'a>(&'a self, stack: &'a Stack, index: usize) -> Option<&'a Expression> {
        match &self.inner {
            CallImpl::AstRef(call) => call.positional_iter().nth(index),
            CallImpl::AstBox(call) => call.positional_iter().nth(index),
            CallImpl::IrRef(call) => call.positional_ast(stack, index).map(|arc| arc.as_ref()),
            CallImpl::IrBox(call) => call.positional_ast(stack, index).map(|arc| arc.as_ref()),
        }
    }
}

impl CallImpl<'_> {
    pub fn to_owned(&self) -> CallImpl<'static> {
        match self {
            CallImpl::AstRef(call) => CallImpl::AstBox(Box::new((*call).clone())),
            CallImpl::AstBox(call) => CallImpl::AstBox(call.clone()),
            CallImpl::IrRef(call) => CallImpl::IrBox(Box::new((*call).clone())),
            CallImpl::IrBox(call) => CallImpl::IrBox(call.clone()),
        }
    }
}

fn ir_has_flag_const(call: &ir::Call, stack: &Stack, flag_name: &str) -> Result<bool, ShellError> {
    Ok(call
        .named_iter(stack)
        .find(|(name, _)| name.item == flag_name)
        .is_some_and(|(_, value)| !matches!(value, Some(Value::Bool { val: false, .. }))))
}

fn ir_get_flag_const<T: FromValue>(
    call: &ir::Call,
    stack: &Stack,
    name: &str,
) -> Result<Option<T>, ShellError> {
    if let Some(val) = call.get_named_arg(stack, name) {
        T::from_value(val.clone()).map(Some)
    } else {
        Ok(None)
    }
}

fn ir_req_const<T: FromValue>(
    call: &ir::Call,
    stack: &Stack,
    head: Span,
    pos: usize,
) -> Result<T, ShellError> {
    let maybe_val = call.positional_nth(stack, pos).cloned();
    let val = maybe_val.ok_or_else(|| {
        let max_idx = call.positional_len(stack).checked_sub(1);
        match max_idx {
            None => ShellError::AccessEmptyContent { span: head },
            Some(max_idx) => ShellError::AccessBeyondEnd {
                max_idx,
                span: head,
            },
        }
    })?;
    T::from_value(val)
}

fn ir_rest_const<T: FromValue>(
    call: &ir::Call,
    stack: &Stack,
    starting_pos: usize,
) -> Result<Vec<T>, ShellError> {
    call.rest_iter_flattened(stack, starting_pos)?
        .into_iter()
        .map(T::from_value)
        .collect()
}

impl<'a> From<&'a ast::Call> for Call<'a> {
    fn from(call: &'a ast::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::AstRef(call),
        }
    }
}

impl<'a> From<&'a ir::Call> for Call<'a> {
    fn from(call: &'a ir::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::IrRef(call),
        }
    }
}
