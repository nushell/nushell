use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Value};

pub struct Length;

impl Command for Length {
    fn name(&self) -> &str {
        "length"
    }

    fn usage(&self) -> &str {
        "Count the number of elements in the input."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("length")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        match input {
            Value::List { val, .. } => {
                let length = val.len();

                Ok(Value::Int {
                    val: length as i64,
                    span: call.head,
                })
            }
            Value::Table { val, .. } => {
                let length = val.len();

                Ok(Value::Int {
                    val: length as i64,
                    span: call.head,
                })
            }
            Value::ValueStream { stream, .. } => {
                let length = stream.count();

                Ok(Value::Int {
                    val: length as i64,
                    span: call.head,
                })
            }
            Value::RowStream { stream, .. } => {
                let length = stream.count();

                Ok(Value::Int {
                    val: length as i64,
                    span: call.head,
                })
            }
            Value::Nothing { .. } => Ok(Value::Int {
                val: 0,
                span: call.head,
            }),
            _ => Ok(Value::Int {
                val: 1,
                span: call.head,
            }),
        }
    }
}
