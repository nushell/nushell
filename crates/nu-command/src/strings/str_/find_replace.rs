use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Spanned;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};
use regex::Regex;
use std::sync::Arc;

struct Arguments {
    all: bool,
    find: String,
    replace: String,
    column_paths: Vec<CellPath>,
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str find-replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("str find-replace")
            .required("find", SyntaxShape::String, "the pattern to find")
            .required("replace", SyntaxShape::String, "the replacement pattern")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally find and replace text by column paths",
            )
            .switch("all", "replace all occurrences of find string", Some('a'))
    }

    fn usage(&self) -> &str {
        "finds and replaces text"
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
                description: "Find and replace contents with capture group",
                example: "'my_library.rb' | str find-replace '(.+).rb' '$1.nu'",
                result: Some(Value::String {
                    val: "my_library.nu".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find string",
                example: "'abc abc abc' | str find-replace -a 'b' 'z'",
                result: Some(Value::String {
                    val: "azc azc azc".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find string in table",
                example:
                    "[[ColA ColB ColC]; [abc abc ads]] | str find-replace -a 'b' 'z' ColA ColC",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::String {
                                val: "azc".to_string(),
                                span: Span::unknown(),
                            },
                            Value::String {
                                val: "abc".to_string(),
                                span: Span::unknown(),
                            },
                            Value::String {
                                val: "ads".to_string(),
                                span: Span::unknown(),
                            },
                        ],
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
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
    let find: Spanned<String> = call.req(engine_state, stack, 0)?;
    let replace: Spanned<String> = call.req(engine_state, stack, 1)?;

    let options = Arc::new(Arguments {
        all: call.has_flag("all"),
        find: find.item,
        replace: replace.item,
        column_paths: call.rest(engine_state, stack, 2)?,
    });
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

struct FindReplace<'a>(&'a str, &'a str);

fn action(
    input: &Value,
    Arguments {
        find, replace, all, ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val, .. } => {
            let FindReplace(find, replacement) = FindReplace(find, replace);
            let regex = Regex::new(find);

            match regex {
                Ok(re) => {
                    if *all {
                        Value::String {
                            val: re.replace_all(val, replacement).to_string(),
                            span: head,
                        }
                    } else {
                        Value::String {
                            val: re.replace(val, replacement).to_string(),
                            span: head,
                        }
                    }
                }
                Err(_) => Value::String {
                    val: val.to_string(),
                    span: head,
                },
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                Span::unknown(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{action, Arguments, SubCommand};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn can_have_capture_groups() {
        let word = Value::String {
            val: "Cargo.toml".to_string(),
            span: Span::unknown(),
        };

        let options = Arguments {
            find: String::from("Cargo.(.+)"),
            replace: String::from("Carga.$1"),
            column_paths: vec![],
            all: false,
        };

        let actual = action(&word, &options, Span::unknown());
        assert_eq!(
            actual,
            Value::String {
                val: "Carga.toml".to_string(),
                span: Span::unknown()
            }
        );
    }
}
