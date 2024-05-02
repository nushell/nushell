use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "into glob"
    }

    fn signature(&self) -> Signature {
        Signature::build("into glob")
            .input_output_types(vec![
                (Type::String, Type::Glob),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Glob)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true) // https://github.com/nushell/nushell/issues/7032
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert data at the given cell paths.",
            )
            .category(Category::Conversions)
    }

    fn usage(&self) -> &str {
        "Convert value to glob."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "text"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        glob_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert string to glob",
                example: "'1234' | into glob",
                result: Some(Value::test_glob("1234")),
            },
            Example {
                description: "convert filepath to glob",
                example: "ls Cargo.toml | get name | into glob",
                result: None,
            },
        ]
    }
}

fn glob_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let cell_paths = call.rest(engine_state, stack, 0)?;
    let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
    let args = Arguments { cell_paths };
    match input {
        PipelineData::ExternalStream { stdout: None, .. } => {
            Ok(Value::glob(String::new(), false, head).into_pipeline_data())
        }
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            // TODO: in the future, we may want this to stream out, converting each to bytes
            let output = stream.into_string()?;
            Ok(Value::glob(output.item, false, head).into_pipeline_data())
        }
        _ => operate(action, args, input, head, engine_state.ctrlc.clone()),
    }
}

fn action(input: &Value, _args: &Arguments, span: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::glob(val.to_string(), false, span),
        x => Value::error(
            ShellError::CantConvert {
                to_type: String::from("glob"),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            },
            span,
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
