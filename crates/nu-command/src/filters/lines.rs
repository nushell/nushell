use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};

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
            .switch("skip-empty", "skip empty lines", Some('s'))
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let skip_empty = call.has_flag("skip-emtpy");
        match input {
            #[allow(clippy::needless_collect)]
            // Collect is needed because the string may not live long enough for
            // the Rc structure to continue using it. If split could take ownership
            // of the split values, then this wouldn't be needed
            PipelineData::Value(Value::String { val, span }, ..) => {
                let lines = val
                    .split(SPLIT_CHAR)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                let iter = lines.into_iter().filter_map(move |s| {
                    if skip_empty && s.is_empty() {
                        None
                    } else {
                        Some(Value::string(s, span))
                    }
                });

                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Stream(stream, ..) => {
                let iter = stream
                    .into_iter()
                    .filter_map(move |value| {
                        if let Value::String { val, span } = value {
                            let inner = val
                                .split(SPLIT_CHAR)
                                .filter_map(|s| {
                                    if skip_empty && s.is_empty() {
                                        None
                                    } else {
                                        Some(Value::String {
                                            val: s.into(),
                                            span,
                                        })
                                    }
                                })
                                .collect::<Vec<Value>>();

                            Some(inner)
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
        }
    }
}
