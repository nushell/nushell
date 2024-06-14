use crate::{PipelineData, RegId, ShellError, Span, Value};

use super::{CallArg, Instruction};

/// Contains the information for a call being made to a declared command.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub registers: &'a [PipelineData],
    pub call_args: &'a [CallArg],
    pub instruction: &'a Instruction,
    pub head: &'a Span,
}

#[derive(Clone, Copy)]
enum ArgRef<'a> {
    Positional(&'a Value),
    Named(&'a str, Option<&'a Value>),
    Spread(&'a Value),
}

impl<'a> ArgRef<'a> {
    fn span(self) -> Span {
        match self {
            ArgRef::Positional(v) => v.span(),
            // FIXME: Named without value needs a span!
            ArgRef::Named(_, v) => v.map(|v| v.span()).unwrap_or(Span::unknown()),
            ArgRef::Spread(v) => v.span(),
        }
    }
}

impl<'a> Call<'a> {
    fn call_args(&self) -> impl Iterator<Item = &CallArg> {
        let Instruction::Call {
            args_start,
            args_len,
            ..
        } = *self.instruction
        else {
            panic!("self.instruction is not Call")
        };
        self.call_args[args_start..(args_start + args_len)].iter()
    }

    fn get_reg_val(&self, reg_id: RegId) -> &Value {
        match &self.registers[reg_id.0 as usize] {
            PipelineData::Value(value, _) => value,
            other => panic!("value in register {reg_id} for argument was not collected: {other:?}"),
        }
    }

    fn arg_refs(&self) -> impl Iterator<Item = ArgRef<'_>> {
        self.call_args().map(|arg| match arg {
            CallArg::Positional(r) => ArgRef::Positional(self.get_reg_val(*r)),
            CallArg::Spread(r) => ArgRef::Spread(self.get_reg_val(*r)),
            CallArg::Flag(name) => ArgRef::Named(&name, None),
            CallArg::Named(name, r) => ArgRef::Named(&name, Some(self.get_reg_val(*r))),
        })
    }

    /// The span encompassing the arguments
    ///
    /// If there are no arguments the span covers where the first argument would exist
    ///
    /// If there are one or more arguments the span encompasses the start of the first argument to
    /// end of the last argument
    pub fn arguments_span(&self) -> Span {
        let past = self.head.past();

        let start = self
            .arg_refs()
            .next()
            .map(|a| a.span())
            .unwrap_or(past)
            .start;
        let end = self.arg_refs().last().map(|a| a.span()).unwrap_or(past).end;

        Span::new(start, end)
    }

    pub fn named_len(&self) -> usize {
        self.call_args()
            .filter(|arg| matches!(arg, CallArg::Named(..) | CallArg::Flag(..)))
            .count()
    }

    pub fn named_iter(&self) -> impl Iterator<Item = (&str, Option<&Value>)> {
        self.arg_refs().filter_map(|arg| match arg {
            ArgRef::Named(name, value) => Some((name, value)),
            _ => None,
        })
    }

    pub fn get_named_arg(&self, flag_name: &str) -> Option<&Value> {
        self.arg_refs().find_map(|arg| match arg {
            ArgRef::Named(name, value) if name == flag_name => value,
            _ => None,
        })
    }

    pub fn positional_len(&self) -> usize {
        self.call_args()
            .filter(|arg| matches!(arg, CallArg::Positional(..)))
            .count()
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Value> {
        self.arg_refs().filter_map(|arg| match arg {
            ArgRef::Positional(value) => Some(value),
            _ => None,
        })
    }

    pub fn positional_nth(&self, index: usize) -> Option<&Value> {
        self.positional_iter().nth(index)
    }

    /// Returns every argument to the rest parameter, as well as whether each argument
    /// is spread or a normal positional argument (true for spread, false for normal)
    pub fn rest_iter(&self, start: usize) -> impl Iterator<Item = (&Value, bool)> {
        self.arg_refs()
            .filter_map(|arg| match arg {
                ArgRef::Positional(value) => Some((value, false)),
                ArgRef::Spread(value) => Some((value, true)),
                _ => None,
            })
            .skip(start)
    }

    pub fn rest_iter_flattened(&self, start: usize) -> Result<Vec<Value>, ShellError> {
        let mut acc = vec![];
        for (rest_val, spread) in self.rest_iter(start) {
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

    pub fn span(&self) -> Span {
        let mut span = *self.head;
        for arg in self.arg_refs() {
            span.end = span.end.max(arg.span().end);
        }
        span
    }
}
