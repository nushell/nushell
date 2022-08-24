use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    levenshtein_distance, Category, Example, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

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
        "compare to strings and return the edit distance/levenshtein distance"
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
        operate(engine_state, stack, call, input)
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

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let compare_string: Spanned<String> = call.req(engine_state, stack, 0)?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, &compare_string.item, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let c = compare_string.item.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &c, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(input: &Value, compare_string: &str, head: Span) -> Value {
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
