use std::cell::RefCell;
use std::rc::Rc;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, EvaluationContext, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, Value, ValueStream};

#[derive(Clone)]
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;
        match input {
            #[allow(clippy::needless_collect)]
            // Collect is needed because the string may not live long enough for
            // the Rc structure to continue using it. If split could take ownership
            // of the split values, then this wouldn't be needed
            PipelineData::Value(Value::String { val, span }) => {
                let lines = val
                    .split(SPLIT_CHAR)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                let iter = lines.into_iter().filter_map(move |s| {
                    if !s.is_empty() {
                        Some(Value::string(s, span))
                    } else {
                        None
                    }
                });

                Ok(iter.into_pipeline_data())
            }
            PipelineData::Stream(stream) => {
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
                    .flatten();

                Ok(iter.into_pipeline_data())
            }
            PipelineData::Value(val) => Err(ShellError::UnsupportedInput(
                format!("Not supported input: {}", val.as_string()?),
                call.head,
            )),
        }
    }
}
