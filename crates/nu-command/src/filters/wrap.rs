use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, Signature, SyntaxShape, Value};

pub struct Wrap;

impl Command for Wrap {
    fn name(&self) -> &str {
        "wrap"
    }

    fn usage(&self) -> &str {
        "Wrap the value into a column."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("wrap").required("name", SyntaxShape::String, "the name of the column")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let span = call.head;
        let name: String = call.req(context, 0)?;

        match input {
            Value::List { vals, .. } => Ok(Value::List {
                vals: vals
                    .into_iter()
                    .map(move |x| Value::Record {
                        cols: vec![name.clone()],
                        vals: vec![x],
                        span,
                    })
                    .collect(),
                span,
            }),
            Value::Stream { stream, .. } => Ok(Value::Stream {
                stream: stream
                    .map(move |x| Value::Record {
                        cols: vec![name.clone()],
                        vals: vec![x],
                        span,
                    })
                    .into_value_stream(),
                span,
            }),
            _ => Ok(Value::Record {
                cols: vec![name],
                vals: vec![input],
                span,
            }),
        }
    }
}
