use crate::RegId;

/// Describes where a `catch` or `finally` handler lives during IR evaluation.
#[derive(Debug, Clone, Copy)]
pub struct ErrorHandler {
    /// Instruction index within the block that will handle the error
    pub handler_index: usize,
    /// Register to put the error information into, when an error occurs
    pub error_register: Option<RegId>,
}

/// An entry on the handler stack maintained while evaluating a `try` expression.
///
/// Entries are pushed in the order they are set up, so the innermost handler is always on top.
/// When control flow unwinds out of a `try` body (an error, `return`, `exit`, `break`, or
/// `continue`), the evaluator pops entries from the top until one takes over.
#[derive(Debug, Clone, Copy)]
pub enum TryHandler {
    /// A `catch` block. Only errors are directed here; other unwinding just discards the entry.
    Catch(ErrorHandler),
    /// A `finally` block that has not run yet. Every kind of unwinding is directed here first,
    /// and resumes once the block finishes.
    Finally(ErrorHandler),
    /// A `finally` block that is currently running. It replaces the [`TryHandler::Finally`]
    /// entry for the duration of the block so that unwinding out of the block itself (for
    /// example a `return` inside `finally`) knows to abandon whatever the block was going to
    /// resume.
    RunningFinally,
}

/// Keeps track of handlers pushed during evaluation of an IR block.
#[derive(Debug, Clone, Default)]
pub struct ErrorHandlerStack {
    handlers: Vec<TryHandler>,
}

impl ErrorHandlerStack {
    pub const fn new() -> ErrorHandlerStack {
        ErrorHandlerStack { handlers: vec![] }
    }

    /// Get the current base of the stack, which establishes a frame.
    pub fn get_base(&self) -> usize {
        self.handlers.len()
    }

    /// Push a new handler onto the stack.
    pub fn push(&mut self, handler: TryHandler) {
        self.handlers.push(handler);
    }

    /// Try to pop a handler from the stack. Won't go below `base`, to avoid retrieving a
    /// handler belonging to a parent frame.
    pub fn pop(&mut self, base: usize) -> Option<TryHandler> {
        if self.handlers.len() > base {
            self.handlers.pop()
        } else {
            None
        }
    }

    /// Reset the stack to the state it was in at the beginning of the frame, in preparation to
    /// return control to the parent frame.
    pub fn leave_frame(&mut self, base: usize) {
        if self.handlers.len() >= base {
            self.handlers.truncate(base);
        } else {
            panic!(
                "ErrorHandlerStack bug: tried to leave frame at {base}, but current base is {}",
                self.get_base()
            )
        }
    }
}
