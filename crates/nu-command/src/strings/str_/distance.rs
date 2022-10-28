use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    levenshtein_distance, Category, Example, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    compare_string: String,
    column_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str distance"
    }

    fn signature(&self) -> Signature {
        Signature::build("str distance")
            .required(
                "compare-string",
                SyntaxShape::String,
                "the first string to compare",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally check if string contains pattern by column paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "compare two strings and return the edit distance/levenshtein distance"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["edit", "match", "score", "levenshtein"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let compare_string: String = call.req(engine_state, stack, 0)?;
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = (!column_paths.is_empty()).then(|| column_paths);
        let args = Arguments {
            compare_string,
            column_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "get the edit distance between two strings",
            example: "'nushell' | str distance 'nutshell'",
            result: Some(Value::Record {
                cols: vec!["distance".to_string()],
                vals: vec![Value::Int {
                    val: 1,
                    span: Span::test_data(),
                }],
                span: Span::test_data(),
            }),
        }]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let compare_string = &args.compare_string;
    match &input {
        Value::String { val, .. } => {
            let distance = levenshtein_distance(val, compare_string);
            Value::Record {
                cols: vec!["distance".to_string()],
                vals: vec![Value::Int {
                    val: distance as i64,
                    span: head,
                }],
                span: head,
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                head,
            ),
        },
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
