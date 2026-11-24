use crate::{DeclId, FromValue, ShellError, Span, Value, ast::Expression, ir};

use super::{Stack, StateWorkingSet, named_flags::call_has_flag};

/// The call that [`Command::run`](super::Command::run) gets: a compiled [`ir::Call`], borrowed or
/// owned.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub head: Span,
    pub decl_id: DeclId,
    pub inner: CallImpl<'a>,
}

#[derive(Debug, Clone)]
pub enum CallImpl<'a> {
    IrRef(&'a ir::Call),
    IrBox(Box<ir::Call>),
}

impl Call<'_> {
    /// Convert the `Call` from any lifetime into `'static`, by cloning the data within onto the
    /// heap.
    pub fn to_owned(&self) -> Call<'static> {
        Call {
            head: self.head,
            decl_id: self.decl_id,
            inner: self.inner.to_owned(),
        }
    }

    // The `*_const` methods read the call's arguments from `stack`. Those that don't need
    // `working_set` keep it anyway, so that the `run_const` implementations calling them don't
    // change.

    /// Check if a boolean flag is set at const-eval time.
    pub fn has_flag_const(
        &self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        let decl = working_set.get_decl(self.decl_id);
        match &self.inner {
            CallImpl::IrRef(call) => call_has_flag(call, stack, decl, flag_name),
            CallImpl::IrBox(call) => call_has_flag(call, stack, decl, flag_name),
        }
    }

    /// Get a typed named argument at const-eval time.
    pub fn get_flag_const<T: FromValue>(
        &self,
        _working_set: &StateWorkingSet,
        stack: &Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        match &self.inner {
            CallImpl::IrRef(call) => ir_get_flag_const(call, stack, name),
            CallImpl::IrBox(call) => ir_get_flag_const(call, stack, name),
        }
    }

    /// Get a required positional argument at const-eval time.
    pub fn req_const<T: FromValue>(
        &self,
        _working_set: &StateWorkingSet,
        stack: &Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        match &self.inner {
            CallImpl::IrRef(call) => ir_req_const(call, stack, self.head, pos),
            CallImpl::IrBox(call) => ir_req_const(call, stack, self.head, pos),
        }
    }

    /// Get the rest of the positional arguments at const-eval time.
    pub fn rest_const<T: FromValue>(
        &self,
        _working_set: &StateWorkingSet,
        stack: &Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        match &self.inner {
            CallImpl::IrRef(call) => ir_rest_const(call, stack, starting_pos),
            CallImpl::IrBox(call) => ir_rest_const(call, stack, starting_pos),
        }
    }

    /// Returns a span covering the call's arguments.
    pub fn arguments_span(&self) -> Span {
        match &self.inner {
            CallImpl::IrRef(call) => call.arguments_span(),
            CallImpl::IrBox(call) => call.arguments_span(),
        }
    }

    /// Returns a span covering the whole call.
    pub fn span(&self) -> Span {
        match &self.inner {
            CallImpl::IrRef(call) => call.span(),
            CallImpl::IrBox(call) => call.span(),
        }
    }

    /// Get a parser info argument by name.
    pub fn get_parser_info<'a>(&'a self, stack: &'a Stack, name: &str) -> Option<&'a Expression> {
        match &self.inner {
            CallImpl::IrRef(call) => call.get_parser_info(stack, name),
            CallImpl::IrBox(call) => call.get_parser_info(stack, name),
        }
    }

    /// Gets all of the positional and spread arguments from `starting_pos` on, flattens spreads,
    /// and returns one list of values.
    pub fn rest_iter_flattened(
        &self,
        stack: &Stack,
        starting_pos: usize,
    ) -> Result<Vec<Value>, ShellError> {
        match &self.inner {
            CallImpl::IrRef(call) => call.rest_iter_flattened(stack, starting_pos),
            CallImpl::IrBox(call) => call.rest_iter_flattened(stack, starting_pos),
        }
    }
}

impl CallImpl<'_> {
    pub fn to_owned(&self) -> CallImpl<'static> {
        match self {
            CallImpl::IrRef(call) => CallImpl::IrBox(Box::new((*call).clone())),
            CallImpl::IrBox(call) => CallImpl::IrBox(call.clone()),
        }
    }
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

impl<'a> From<&'a ir::Call> for Call<'a> {
    fn from(call: &'a ir::Call) -> Self {
        Call {
            head: call.head,
            decl_id: call.decl_id,
            inner: CallImpl::IrRef(call),
        }
    }
}
