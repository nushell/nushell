use nu_protocol::{
    FromValue, ShellError, Span, Value,
    engine::{self, EngineState, Stack, StateWorkingSet, named_flags::call_has_flag},
    ir,
};

pub trait CallExt {
    /// Check if a boolean flag is set (i.e. `--bool` or `--bool=true`). A switch given a value that
    /// isn't a bool is an error.
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
        stack: &Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError>;

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError>;

    /// True if the command has any positional or rest arguments, excluding before the given index.
    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool;
}

impl CallExt for ir::Call {
    fn has_flag(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        flag_name: &str,
    ) -> Result<bool, ShellError> {
        call_has_flag(self, stack, engine_state.get_decl(self.decl_id), flag_name)
    }

    fn get_flag<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        name: &str,
    ) -> Result<Option<T>, ShellError> {
        // Null flags that are not type-accepted are dropped in normalize_call_arguments.
        // Remaining null values are intentional (flag type accepts nothing).
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
        engine_state: &EngineState,
        stack: &mut Stack,
        starting_pos: usize,
    ) -> Result<Vec<T>, ShellError> {
        self.rest_iter_flattened(stack, starting_pos)?
            .into_iter()
            .map(|value| {
                T::from_value(value.clone())
                    .map_err(|err| explain_list_argument(err, &value, engine_state))
            })
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
            .filter(|v| !v.is_nothing())
            .cloned()
            .map(T::from_value)
            .transpose()
    }

    fn opt_const<T: FromValue>(
        &self,
        _working_set: &StateWorkingSet,
        stack: &Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        self.positional_iter(stack)
            .nth(pos)
            .filter(|v| !v.is_nothing())
            .cloned()
            .map(T::from_value)
            .transpose()
    }

    fn req<T: FromValue>(
        &self,
        _engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        let maybe_val = self.positional_iter(stack).nth(pos).cloned();
        let val = maybe_val.ok_or_else(|| {
            let max_idx = self.positional_iter(stack).count().checked_sub(1);
            match max_idx {
                None => ShellError::AccessEmptyContent { span: self.head },
                Some(max_idx) => ShellError::AccessBeyondEnd {
                    max_idx,
                    span: self.head,
                },
            }
        })?;

        T::from_value(val)
    }

    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool {
        self.rest_iter(stack, starting_pos).next().is_some()
    }
}

macro_rules! proxy {
    ($self:ident . $method:ident ($($param:expr),*)) => (match &$self.inner {
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
        stack: &Stack,
        pos: usize,
    ) -> Result<Option<T>, ShellError> {
        proxy!(self.opt_const(working_set, stack, pos))
    }

    fn req<T: FromValue>(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        pos: usize,
    ) -> Result<T, ShellError> {
        proxy!(self.req(engine_state, stack, pos))
    }

    fn has_positional_args(&self, stack: &Stack, starting_pos: usize) -> bool {
        proxy!(self.has_positional_args(stack, starting_pos))
    }
}

/// Lists are not spread automatically, so a list given to a rest parameter fails to convert
/// into a single argument. Point that error at the spread operator, naming the argument that
/// has to be spread.
fn explain_list_argument(err: ShellError, value: &Value, engine_state: &EngineState) -> ShellError {
    if !matches!(value, Value::List { .. }) {
        return err;
    }

    let mut err = err;
    if let ShellError::CantConvert { help, .. } = &mut err
        && help.is_none()
    {
        let arg = String::from_utf8_lossy(engine_state.get_span_contents(value.span()));
        let arg = arg.trim();
        let arg = if arg.is_empty() { "$list" } else { arg };
        *help = Some(format!(
            "to pass each item of the list as a separate argument, spread it like so: ...{arg}"
        ));
    }
    err
}
