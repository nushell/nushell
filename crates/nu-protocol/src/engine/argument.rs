use crate::{Span, Value};

/// Represents a fully evaluated argument to a call.
#[derive(Debug, Clone)]
pub enum Argument {
    /// A positional argument
    Positional {
        span: Span,
        val: Value,
    },
    Spread {
        span: Span,
        vals: Value,
    },
    /// A named argument with no value, e.g. `--flag`
    Flag {
        name: Box<str>,
        span: Span,
    },
    /// A named argument with a value, e.g. `--flag value` or `--flag=`
    Named {
        name: Box<str>,
        span: Span,
        val: Value,
    },
}

impl Argument {
    /// The span encompassing the argument's usage within the call, distinct from the span of the
    /// actual value of the argument.
    pub fn span(&self) -> Span {
        match self {
            Argument::Positional { span, .. } => *span,
            Argument::Spread { span, .. } => *span,
            Argument::Flag { span, .. } => *span,
            Argument::Named { span, .. } => *span,
        }
    }
}

/// Stores the argument context for calls in IR evaluation.
#[derive(Debug, Clone)]
pub struct ArgumentStack {
    arguments: Vec<Argument>,
}

impl ArgumentStack {
    /// Create a new, empty argument stack.
    pub const fn new() -> Self {
        ArgumentStack { arguments: vec![] }
    }

    /// Returns the index of the end of the argument stack. Call and save this before adding
    /// arguments.
    pub fn get_base(&self) -> usize {
        self.arguments.len()
    }

    /// Calculates the number of arguments past the given [previously retrieved](.get_base) base
    /// pointer.
    pub fn get_len(&self, base: usize) -> usize {
        self.arguments.len().checked_sub(base).unwrap_or_else(|| {
            panic!(
                "base ({}) is beyond the end of the arguments stack ({})",
                base,
                self.arguments.len()
            );
        })
    }

    /// Push an argument onto the end of the argument stack.
    pub fn push(&mut self, argument: Argument) {
        self.arguments.push(argument);
    }

    /// Clear all of the arguments after the given base index, to prepare for the next frame.
    pub fn leave_frame(&mut self, base: usize) {
        self.arguments.truncate(base);
    }

    /// Get arguments for the frame based on the given [`base`](`.get_base()`) and
    /// [`len`](`.get_len()`) parameters.
    pub fn get_args(&self, base: usize, len: usize) -> &[Argument] {
        &self.arguments[base..(base + len)]
    }
}
