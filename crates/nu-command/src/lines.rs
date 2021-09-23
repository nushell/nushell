use std::cell::RefCell;
use std::rc::Rc;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Span, Value, ValueStream};

pub struct Lines;

const SPLIT_CHAR: char = '\n';

impl Command for Lines {
    fn name(&self) -> &str {
        "lines"
    }

    fn usage(&self) -> &str {
        "Converts input to lines"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("lines")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        _call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let value = match input {
            Value::String { val, span } => {
                let iter = val
                    .split(SPLIT_CHAR)
                    .map(|s| Value::String {
                        val: s.into(),
                        span,
                    })
                    .collect::<Vec<Value>>(); // <----- how to avoid collecting?

                Value::Stream {
                    stream: ValueStream(Rc::new(RefCell::new(iter.into_iter()))),
                    span: Span::unknown(),
                }
            }
            Value::Stream { stream, span: _ } => {
                let iter = stream
                    .into_iter()
                    .filter_map(|value| {
                        if let Value::String { val, span } = value {
                            let inner = val
                                .split(SPLIT_CHAR)
                                .filter_map(|s| {
                                    if !s.is_empty() {
                                        Some(Value::String {
                                            val: s.into(),
                                            span,
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<Value>>();

                            Some(inner)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<Value>>(); // <----- how to avoid collecting?

                Value::Stream {
                    stream: ValueStream(Rc::new(RefCell::new(iter.into_iter()))),
                    span: Span::unknown(),
                }
            }
            _ => unimplemented!(),
        };

        Ok(value)
    }
}
