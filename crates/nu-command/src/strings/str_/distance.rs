use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{engine::StateWorkingSet, levenshtein_distance};

#[derive(Clone)]
pub struct StrDistance;

struct Arguments {
    compare_string: String,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl Command for StrDistance {
    fn name(&self) -> &str {
        "str distance"
    }

    fn signature(&self) -> Signature {
        Signature::build("str distance")
            .input_output_types(vec![
                (Type::String, Type::Int),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .required(
                "compare-string",
                SyntaxShape::String,
                "The first string to compare.",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Compare two strings and return the edit distance/Levenshtein distance."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["edit", "levenshtein"]
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
        let compare_string: String = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            compare_string,
            cell_paths,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let compare_string: String = call.req_const(working_set, 0)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            compare_string,
            cell_paths,
        };
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
                description: "get the edit distance between two strings",
                example: "'nushell' | str distance 'nutshell'",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Compute edit distance between strings in table and another string, using cell paths",
                example: "[{a: 'nutshell' b: 'numetal'}] | str distance 'nushell' 'a' 'b'",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(4),
                })])),
            },
            Example {
                description: "Compute edit distance between strings in record and another string, using cell paths",
                example: "{a: 'nutshell' b: 'numetal'} | str distance 'nushell' a b",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(4),
                })),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let compare_string = &args.compare_string;
    match input {
        Value::String { val, .. } => {
            let distance = levenshtein_distance(val, compare_string);
            Value::int(distance as i64, head)
        }
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

        test_examples(StrDistance {})
    }
}
