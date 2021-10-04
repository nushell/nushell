use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, Signature, SyntaxShape, Value};

pub struct Ls;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn usage(&self) -> &str {
        "List the files in a directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls").optional(
            "pattern",
            SyntaxShape::GlobPattern,
            "the glob pattern to use",
        )
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let pattern = if let Some(expr) = call.positional.get(0) {
            let result = eval_expression(context, expr)?;
            let mut result = result.as_string()?;

            let path = std::path::Path::new(&result);
            if path.is_dir() {
                result.push('*');
            }

            result
        } else {
            "*".into()
        };

        let call_span = call.head;
        let glob = glob::glob(&pattern).unwrap();

        Ok(Value::Stream {
            stream: glob
                .into_iter()
                .map(move |x| match x {
                    Ok(path) => match std::fs::symlink_metadata(&path) {
                        Ok(metadata) => {
                            let is_file = metadata.is_file();
                            let is_dir = metadata.is_dir();
                            let filesize = metadata.len();

                            Value::Record {
                                cols: vec!["name".into(), "type".into(), "size".into()],
                                vals: vec![
                                    Value::String {
                                        val: path.to_string_lossy().to_string(),
                                        span: call_span,
                                    },
                                    if is_file {
                                        Value::string("file", call_span)
                                    } else if is_dir {
                                        Value::string("dir", call_span)
                                    } else {
                                        Value::Nothing { span: call_span }
                                    },
                                    Value::Filesize {
                                        val: filesize,
                                        span: call_span,
                                    },
                                ],
                                span: call_span,
                            }
                        }
                        Err(_) => Value::Record {
                            cols: vec!["name".into(), "type".into(), "size".into()],
                            vals: vec![
                                Value::String {
                                    val: path.to_string_lossy().to_string(),
                                    span: call_span,
                                },
                                Value::Nothing { span: call_span },
                                Value::Nothing { span: call_span },
                            ],
                            span: call_span,
                        },
                    },
                    _ => Value::Nothing { span: call_span },
                })
                .into_value_stream(),
            span: call_span,
        })
    }
}
