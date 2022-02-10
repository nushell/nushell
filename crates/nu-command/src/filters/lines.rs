use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Lines;

impl Command for Lines {
    fn name(&self) -> &str {
        "lines"
    }

    fn usage(&self) -> &str {
        "Converts input to lines"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("lines")
            .switch("skip_empty", "skip empty lines", Some('s'))
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let skip_empty = call.has_flag("skip_empty");
        match input {
            #[allow(clippy::needless_collect)]
            // Collect is needed because the string may not live long enough for
            // the Rc structure to continue using it. If split could take ownership
            // of the split values, then this wouldn't be needed
            PipelineData::Value(Value::String { val, span }, ..) => {
                let split_char = if val.contains("\r\n") { "\r\n" } else { "\n" };

                let mut lines = val
                    .split(split_char)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                // if the last one is empty, remove it, as it was just
                // a newline at the end of the input we got
                if let Some(last) = lines.last() {
                    if last.is_empty() {
                        lines.pop();
                    }
                }

                let iter = lines.into_iter().filter_map(move |s| {
                    if skip_empty && s.trim().is_empty() {
                        None
                    } else {
                        Some(Value::string(s, span))
                    }
                });

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::ListStream(stream, ..) => {
                let mut split_char = "\n";

                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
                        if let Value::String { val, span } = value {
                            if split_char != "\r\n" && val.contains("\r\n") {
                                split_char = "\r\n";
                            }

                            let mut lines = val
                                .split(split_char)
                                .filter_map(|s| {
                                    if skip_empty && s.trim().is_empty() {
                                        None
                                    } else {
                                        Some(s.to_string())
                                    }
                                })
                                .collect::<Vec<_>>();

                            // if the last one is empty, remove it, as it was just
                            // a newline at the end of the input we got
                            if let Some(last) = lines.last() {
                                if last.is_empty() {
                                    lines.pop();
                                }
                            }

                            Some(
                                lines
                                    .into_iter()
                                    .map(move |x| Value::String { val: x, span }),
                            )
                        } else {
                            None
                        }
                    })
                    .flatten();

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Value(val, ..) => Err(ShellError::UnsupportedInput(
                format!("Not supported input: {}", val.as_string()?),
                call.head,
            )),
            PipelineData::RawStream(..) => {
                let config = stack.get_config()?;

                //FIXME: Make sure this can fail in the future to let the user
                //know to use a different encoding
                let s = input.collect_string("", &config)?;

                let split_char = if s.contains("\r\n") { "\r\n" } else { "\n" };

                #[allow(clippy::needless_collect)]
                let mut lines = s
                    .split(split_char)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                // if the last one is empty, remove it, as it was just
                // a newline at the end of the input we got
                if let Some(last) = lines.last() {
                    if last.is_empty() {
                        lines.pop();
                    }
                }

                let iter = lines.into_iter().filter_map(move |s| {
                    if skip_empty && s.trim().is_empty() {
                        None
                    } else {
                        Some(Value::string(s, head))
                    }
                });

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
        }
    }
}
