use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, Example, PipelineData, PipelineMetadata, Signature, Value,
};

#[derive(Clone)]
pub struct Metadata;

impl Command for Metadata {
    fn name(&self) -> &str {
        "metadata"
    }

    fn usage(&self) -> &str {
        "Get the metadata for items in the stream"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("metadata").category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();

        let metadata = input.metadata();

        input.map(
            move |x| {
                let span = x.span();

                let mut cols = vec![];
                let mut vals = vec![];

                cols.push("span".into());
                if let Ok(span) = span {
                    vals.push(Value::Record {
                        cols: vec!["start".into(), "end".into()],
                        vals: vec![
                            Value::Int {
                                val: span.start as i64,
                                span,
                            },
                            Value::Int {
                                val: span.end as i64,
                                span,
                            },
                        ],
                        span: head,
                    });
                }

                if let Some(x) = &metadata {
                    match x {
                        PipelineMetadata {
                            data_source: DataSource::Ls,
                        } => {
                            cols.push("source".into());
                            vals.push(Value::String {
                                val: "ls".into(),
                                span: head,
                            })
                        }
                    }
                }

                Value::Record {
                    cols,
                    vals,
                    span: head,
                }
            },
            ctrlc,
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the metadata of a value",
            example: "3 | metadata",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Metadata {})
    }
}
