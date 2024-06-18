use crate::{
    engine::{Argument, Stack},
    ShellError, Span, Value,
};

/// Contains the information for a call being made to a declared command.
#[derive(Debug, Clone)]
pub struct Call {
    /// The declaration ID of the command to be invoked.
    pub decl_id: usize,
    /// The span encompassing the command name, before the arguments.
    pub head: Span,
    /// The base index of the arguments for this call within the
    /// [argument stack](crate::engine::ArgumentStack).
    pub args_base: usize,
    /// The number of [`Argument`]s for the call. Note that this just counts the number of
    /// `Argument` entries on the stack, and has nothing to do with the actual number of positional
    /// or spread arguments.
    pub args_len: usize,
}

impl Call {
    /// Get the arguments for this call from the arguments stack.
    pub fn arguments<'a>(&self, stack: &'a Stack) -> &'a [Argument] {
        stack.argument_stack.get_args(self.args_base, self.args_len)
    }

    /// The span encompassing the arguments
    ///
    /// If there are no arguments the span covers where the first argument would exist
    ///
    /// If there are one or more arguments the span encompasses the start of the first argument to
    /// end of the last argument
    pub fn arguments_span(&self, stack: &Stack) -> Span {
        let past = self.head.past();

        let args = self.arguments(stack);

        let start = args.first().map(|a| a.span()).unwrap_or(past).start;
        let end = args.last().map(|a| a.span()).unwrap_or(past).end;

        Span::new(start, end)
    }

    pub fn named_len(&self, stack: &Stack) -> usize {
        self.arguments(stack)
            .iter()
            .filter(|arg| matches!(arg, Argument::Named { .. } | Argument::Flag { .. }))
            .count()
    }

    pub fn named_iter<'a>(
        &self,
        stack: &'a Stack,
    ) -> impl Iterator<Item = (&'a str, Option<&'a Value>)> + 'a {
        self.arguments(stack).iter().filter_map(
            |arg: &Argument| -> Option<(&str, Option<&Value>)> {
                match arg {
                    Argument::Flag { name, .. } => Some((&name, None)),
                    Argument::Named { name, val, .. } => Some((&name, Some(val))),
                    _ => None,
                }
            },
        )
    }

    pub fn get_named_arg<'a>(&self, stack: &'a Stack, flag_name: &str) -> Option<&'a Value> {
        self.named_iter(stack)
            .find_map(|(name, val)| (name == flag_name).then_some(val))
            .flatten()
    }

    pub fn positional_len(&self, stack: &Stack) -> usize {
        self.arguments(stack)
            .iter()
            .filter(|arg| matches!(arg, Argument::Positional { .. }))
            .count()
    }

    pub fn positional_iter<'a>(&self, stack: &'a Stack) -> impl Iterator<Item = &'a Value> {
        self.arguments(stack).iter().filter_map(|arg| match arg {
            Argument::Positional { val, .. } => Some(val),
            _ => None,
        })
    }

    pub fn positional_nth<'a>(&self, stack: &'a Stack, index: usize) -> Option<&'a Value> {
        self.positional_iter(stack).nth(index)
    }

    /// Returns every argument to the rest parameter, as well as whether each argument
    /// is spread or a normal positional argument (true for spread, false for normal)
    pub fn rest_iter<'a>(
        &self,
        stack: &'a Stack,
        start: usize,
    ) -> impl Iterator<Item = (&'a Value, bool)> + 'a {
        self.arguments(stack)
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional { val, .. } => Some((val, false)),
                Argument::Spread { vals, .. } => Some((vals, true)),
                _ => None,
            })
            .skip(start)
    }

    pub fn rest_iter_flattened(
        &self,
        stack: &Stack,
        start: usize,
    ) -> Result<Vec<Value>, ShellError> {
        let mut acc = vec![];
        for (rest_val, spread) in self.rest_iter(stack, start) {
            if spread {
                match rest_val {
                    Value::List { vals, .. } => acc.extend(vals.iter().cloned()),
                    Value::Error { error, .. } => return Err(ShellError::clone(error)),
                    _ => {
                        return Err(ShellError::CannotSpreadAsList {
                            span: rest_val.span(),
                        })
                    }
                }
            } else {
                acc.push(rest_val.clone());
            }
        }
        Ok(acc)
    }

    pub fn span(&self, stack: &Stack) -> Span {
        let mut span = self.head;
        for arg in self.arguments(stack).iter() {
            span.end = span.end.max(arg.span().end);
        }
        span
    }
}
