use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};
use std::sync::Arc;

struct Arguments {
    length: Option<i64>,
    character: Option<String>,
    column_paths: Vec<CellPath>,
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str rpad"
    }

    fn signature(&self) -> Signature {
        Signature::build("str rpad")
            .required_named("length", SyntaxShape::Int, "length to pad to", Some('l'))
            .required_named(
                "character",
                SyntaxShape::String,
                "character to pad with",
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally check if string contains pattern by column paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Right-pad a string to a specific length"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["append", "truncate", "padding"]
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
        vec![
            Example {
                description: "Right-pad a string with asterisks until it's 10 characters wide",
                example: "'nushell' | str rpad -l 10 -c '*'",
                result: Some(Value::String {
                    val: "nushell***".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Right-pad a string with zeroes until it's 10 characters wide",
                example: "'123' | str rpad -l 10 -c '0'",
                result: Some(Value::String {
                    val: "1230000000".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Use rpad to truncate a string to its first three characters",
                example: "'123456789' | str rpad -l 3 -c '0'",
                result: Some(Value::String {
                    val: "123".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Use rpad to pad Unicode",
                example: "'▉' | str rpad -l 10 -c '▉'",
                result: Some(Value::String {
                    val: "▉▉▉▉▉▉▉▉▉▉".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let options = Arc::new(Arguments {
        length: call.get_flag(engine_state, stack, "length")?,
        character: call.get_flag(engine_state, stack, "character")?,
        column_paths: call.rest(engine_state, stack, 0)?,
    });

    if options.length.expect("this exists") < 0 {
        return Err(ShellError::UnsupportedInput(
            String::from("The length of the string cannot be negative"),
            call.head,
        ));
    }

    let head = call.head;
    input.map(
        move |v| {
            if options.column_paths.is_empty() {
                action(&v, &options, head)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let opt = options.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &opt, head)),
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

fn action(
    input: &Value,
    Arguments {
        character, length, ..
    }: &Arguments,
    head: Span,
) -> Value {
    match &input {
        Value::String { val, .. } => match length {
            Some(x) => {
                let s = *x as usize;
                if s < val.len() {
                    Value::String {
                        val: val.chars().take(s).collect::<String>(),
                        span: head,
                    }
                } else {
                    let mut res = val.to_string();
                    res += &character.as_ref().expect("we already know this flag needs to exist because the command is type checked before we call the action function").repeat(s - val.chars().count());
                    Value::String {
                        val: res,
                        span: head,
                    }
                }
            }
            None => Value::Error {
                error: ShellError::UnsupportedInput(
                    String::from("Length argument is missing"),
                    head,
                ),
            },
        },
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
