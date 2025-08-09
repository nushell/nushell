use std::sync::Arc;

use crate::{
    DeclId, ShellError, Span, Spanned, Value,
    ast::Expression,
    engine::{self, Argument, Stack},
};

use super::DataSlice;

/// Contains the information for a call being made to a declared command.
#[derive(Debug, Clone)]
pub struct Call {
    /// The declaration ID of the command to be invoked.
    pub decl_id: DeclId,
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
    pub fn build(decl_id: DeclId, head: Span) -> CallBuilder {
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
        stack.arguments.get_args(self.args_base, self.args_len)
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

    /// The number of named arguments, with or without values.
    pub fn named_len(&self, stack: &Stack) -> usize {
        self.arguments(stack)
            .iter()
            .filter(|arg| matches!(arg, Argument::Named { .. } | Argument::Flag { .. }))
            .count()
    }

    /// Iterate through named arguments, with or without values.
    pub fn named_iter<'a>(
        &'a self,
        stack: &'a Stack,
    ) -> impl Iterator<Item = (Spanned<&'a str>, Option<&'a Value>)> + 'a {
        self.arguments(stack).iter().filter_map(
            |arg: &Argument| -> Option<(Spanned<&str>, Option<&Value>)> {
                match arg {
                    Argument::Flag {
                        data, name, span, ..
                    } => Some((
                        Spanned {
                            item: std::str::from_utf8(&data[*name]).expect("invalid arg name"),
                            span: *span,
                        },
                        None,
                    )),
                    Argument::Named {
                        data,
                        name,
                        span,
                        val,
                        ..
                    } => Some((
                        Spanned {
                            item: std::str::from_utf8(&data[*name]).expect("invalid arg name"),
                            span: *span,
                        },
                        Some(val),
                    )),
                    _ => None,
                }
            },
        )
    }

    /// Get a named argument's value by name. Returns [`None`] for named arguments with no value as
    /// well.
    pub fn get_named_arg<'a>(&self, stack: &'a Stack, flag_name: &str) -> Option<&'a Value> {
        // Optimized to avoid str::from_utf8()
        self.arguments(stack)
            .iter()
            .find_map(|arg: &Argument| -> Option<Option<&Value>> {
                match arg {
                    Argument::Flag { data, name, .. } if &data[*name] == flag_name.as_bytes() => {
                        Some(None)
                    }
                    Argument::Named {
                        data, name, val, ..
                    } if &data[*name] == flag_name.as_bytes() => Some(Some(val)),
                    _ => None,
                }
            })
            .flatten()
    }

    /// The number of positional arguments, excluding spread arguments.
    pub fn positional_len(&self, stack: &Stack) -> usize {
        self.arguments(stack)
            .iter()
            .filter(|arg| matches!(arg, Argument::Positional { .. }))
            .count()
    }

    /// Iterate through positional arguments. Does not include spread arguments.
    pub fn positional_iter<'a>(&self, stack: &'a Stack) -> impl Iterator<Item = &'a Value> {
        self.arguments(stack).iter().filter_map(|arg| match arg {
            Argument::Positional { val, .. } => Some(val),
            _ => None,
        })
    }

    /// Get a positional argument by index. Does not include spread arguments.
    pub fn positional_nth<'a>(&self, stack: &'a Stack, index: usize) -> Option<&'a Value> {
        self.positional_iter(stack).nth(index)
    }

    /// Get the AST node for a positional argument by index. Not usually available unless the decl
    /// required it.
    pub fn positional_ast<'a>(
        &self,
        stack: &'a Stack,
        index: usize,
    ) -> Option<&'a Arc<Expression>> {
        self.arguments(stack)
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional { ast, .. } => Some(ast),
                _ => None,
            })
            .nth(index)
            .and_then(|option| option.as_ref())
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

    /// Returns all of the positional arguments including and after `start`, with spread arguments
    /// flattened into a single `Vec`.
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
                    Value::Nothing { .. } => (),
                    Value::Error { error, .. } => return Err(ShellError::clone(error)),
                    _ => {
                        return Err(ShellError::CannotSpreadAsList {
                            span: rest_val.span(),
                        });
                    }
                }
            } else {
                acc.push(rest_val.clone());
            }
        }
        Ok(acc)
    }

    /// Get a parser info argument by name.
    pub fn get_parser_info<'a>(&self, stack: &'a Stack, name: &str) -> Option<&'a Expression> {
        self.arguments(stack)
            .iter()
            .find_map(|argument| match argument {
                Argument::ParserInfo {
                    data,
                    name: name_slice,
                    info: expr,
                } if &data[*name_slice] == name.as_bytes() => Some(expr.as_ref()),
                _ => None,
            })
    }

    /// Returns a span encompassing the entire call.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Resets the [`Stack`] to its state before the call was made.
    pub fn leave(&self, stack: &mut Stack) {
        stack.arguments.leave_frame(self.args_base);
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
            self.inner.args_base = stack.arguments.get_base();
        }
        self.inner.args_len += 1;
        if let Some(span) = argument.span() {
            self.inner.span = self.inner.span.merge(span);
        }
        stack.arguments.push(argument);
        self
    }

    /// Add a positional argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_positional(&mut self, stack: &mut Stack, span: Span, val: Value) -> &mut Self {
        self.add_argument(
            stack,
            Argument::Positional {
                span,
                val,
                ast: None,
            },
        )
    }

    /// Add a spread argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_spread(&mut self, stack: &mut Stack, span: Span, vals: Value) -> &mut Self {
        self.add_argument(
            stack,
            Argument::Spread {
                span,
                vals,
                ast: None,
            },
        )
    }

    /// Add a flag (no-value named) argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_flag(
        &mut self,
        stack: &mut Stack,
        name: impl AsRef<str>,
        short: impl AsRef<str>,
        span: Span,
    ) -> &mut Self {
        let (data, name, short) = data_from_name_and_short(name.as_ref(), short.as_ref());
        self.add_argument(
            stack,
            Argument::Flag {
                data,
                name,
                short,
                span,
            },
        )
    }

    /// Add a named argument to the [`Stack`] and reference it from the [`Call`].
    pub fn add_named(
        &mut self,
        stack: &mut Stack,
        name: impl AsRef<str>,
        short: impl AsRef<str>,
        span: Span,
        val: Value,
    ) -> &mut Self {
        let (data, name, short) = data_from_name_and_short(name.as_ref(), short.as_ref());
        self.add_argument(
            stack,
            Argument::Named {
                data,
                name,
                short,
                span,
                val,
                ast: None,
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

fn data_from_name_and_short(name: &str, short: &str) -> (Arc<[u8]>, DataSlice, DataSlice) {
    let data: Vec<u8> = name.bytes().chain(short.bytes()).collect();
    let data: Arc<[u8]> = data.into();
    let name = DataSlice {
        start: 0,
        len: name.len().try_into().expect("flag name too big"),
    };
    let short = DataSlice {
        start: name.start.checked_add(name.len).expect("flag name too big"),
        len: short.len().try_into().expect("flag short name too big"),
    };
    (data, name, short)
}
