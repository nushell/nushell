use crate::{
    engine::{self, Argument, Stack},
    ShellError, Span, Spanned, Value,
};

/// Contains the information for a call being made to a declared command.
#[derive(Debug, Clone)]
pub struct Call {
    /// The declaration ID of the command to be invoked.
    pub decl_id: usize,
    /// The span encompassing the command name, before the arguments.
    pub head: Span,
    /// The span encompassing the command name and all arguments.
    pub span: Span,
    /// The base index of the arguments for this call within the
    /// [argument stack](crate::engine::ArgumentStack).
    pub args_base: usize,
    /// The number of [`Argument`]s for the call. Note that this just counts the number of
    /// `Argument` entries on the stack, and has nothing to do with the actual number of positional
    /// or spread arguments.
    pub args_len: usize,
}

impl Call {
    /// Build a new call with arguments.
    pub fn build(decl_id: usize, head: Span) -> CallBuilder {
        CallBuilder {
            inner: Call {
                decl_id,
                head,
                span: head,
                args_base: 0,
                args_len: 0,
            },
        }
    }

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
    pub fn arguments_span(&self) -> Span {
        let past = self.head.past();
        Span::new(past.start, self.span.end)
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
    ) -> impl Iterator<Item = (Spanned<&'a str>, Option<&'a Value>)> + 'a {
        self.arguments(stack).iter().filter_map(
            |arg: &Argument| -> Option<(Spanned<&str>, Option<&Value>)> {
                match arg {
                    Argument::Flag { name, span, .. } => Some((
                        Spanned {
                            item: name,
                            span: *span,
                        },
                        None,
                    )),
                    Argument::Named {
                        name, span, val, ..
                    } => Some((
                        Spanned {
                            item: name,
                            span: *span,
                        },
                        Some(val),
                    )),
                    _ => None,
                }
            },
        )
    }

    pub fn get_named_arg<'a>(&self, stack: &'a Stack, flag_name: &str) -> Option<&'a Value> {
        self.named_iter(stack)
            .find_map(|(name, val)| (name.item == flag_name).then_some(val))
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

    /// Returns a span encompassing the entire call.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Resets the [`Stack`] to its state before the call was made.
    pub fn leave(&self, stack: &mut Stack) {
        stack.argument_stack.leave_frame(self.args_base);
    }
}

/// Utility struct for building a [`Call`] with arguments on the [`Stack`].
pub struct CallBuilder {
    inner: Call,
}

impl CallBuilder {
    /// Add an argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_argument(&mut self, stack: &mut Stack, argument: Argument) -> &mut Self {
        if self.inner.args_len == 0 {
            self.inner.args_base = stack.argument_stack.get_base();
        }
        self.inner.args_len += 1;
        self.inner.span = self.inner.span.append(argument.span());
        stack.argument_stack.push(argument);
        self
    }

    /// Add a positional argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_positional(&mut self, stack: &mut Stack, span: Span, val: Value) -> &mut Self {
        self.add_argument(stack, Argument::Positional { span, val })
    }

    /// Add a spread argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_spread(&mut self, stack: &mut Stack, span: Span, vals: Value) -> &mut Self {
        self.add_argument(stack, Argument::Spread { span, vals })
    }

    /// Add a flag (no-value named) argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_flag(&mut self, stack: &mut Stack, name: impl AsRef<str>, span: Span) -> &mut Self {
        self.add_argument(
            stack,
            Argument::Flag {
                name: name.as_ref().into(),
                span,
            },
        )
    }

    /// Add a named argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_named(
        &mut self,
        stack: &mut Stack,
        name: impl AsRef<str>,
        span: Span,
        val: Value,
    ) -> &mut Self {
        self.add_argument(
            stack,
            Argument::Named {
                name: name.as_ref().into(),
                span,
                val,
            },
        )
    }

    /// Produce the finished [`Call`] from the builder.
    ///
    /// The call should be entered / run before any other calls are constructed, because the
    /// argument stack will be reset when they exit.
    pub fn finish(&self) -> Call {
        self.inner.clone()
    }

    /// Run a closure with the [`Call`] as an [`engine::Call`] reference, and then clean up the
    /// arguments that were added to the [`Stack`] after.
    ///
    /// For convenience. Calls [`Call::leave`] after the closure ends.
    pub fn with<T>(
        self,
        stack: &mut Stack,
        f: impl FnOnce(&mut Stack, &engine::Call<'_>) -> T,
    ) -> T {
        let call = engine::Call::from(&self.inner);
        let result = f(stack, &call);
        self.inner.leave(stack);
        result
    }
}
