use deunicode::deunicode;
use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str deunicode"
    }

    fn signature(&self) -> Signature {
        Signature::build("str deunicode")
            .input_output_types(vec![(Type::String, Type::String)])
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Convert Unicode string to pure ASCII."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "ascii"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);

        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);

        operate(
            action,
            args,
            input,
            call.head,
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "deunicode a string",
            example: "'A…B' | str deunicode",
            result: Some(Value::test_string("A...B")),
        }]
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::string(deunicode(val), head),
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
