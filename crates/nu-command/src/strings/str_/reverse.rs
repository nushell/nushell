use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrReverse;

impl Command for StrReverse {
    fn name(&self) -> &str {
        "str reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("str reverse")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, reverse strings at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Reverse every string in the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "inverse", "flip"]
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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Reverse a single string",
                example: "'Nushell' | str reverse",
                result: Some(Value::test_string("llehsuN")),
            },
            Example {
                description: "Reverse multiple strings in a list",
                example: "['Nushell' 'is' 'cool'] | str reverse",
                result: Some(Value::list(
                    vec![
                        Value::test_string("llehsuN"),
                        Value::test_string("si"),
                        Value::test_string("looc"),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn action(input: &Value, _arg: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::string(val.chars().rev().collect::<String>(), head),
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

        test_examples(StrReverse {})
    }
}
