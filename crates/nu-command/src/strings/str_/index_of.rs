use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::Spanned;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};
use std::sync::Arc;

struct Arguments {
    end: bool,
    pattern: String,
    range: Option<Value>,
    column_paths: Vec<CellPath>,
}

#[derive(Clone)]
pub struct SubCommand;

#[derive(Clone)]
pub struct IndexOfOptionalBounds(i32, i32);

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("str index-of")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to find index of",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally returns index of pattern in string by column paths",
            )
            .named(
                "range",
                SyntaxShape::Any,
                "optional start and/or end index",
                Some('r'),
            )
            .switch("end", "search from the end of the string", Some('e'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Returns start index of first occurrence of pattern in string, or -1 if no match"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "find", "search", "index"]
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
                description: "Returns index of pattern in string",
                example: " 'my_library.rb' | str index-of '.rb'",
                result: Some(Value::test_int(10)),
            },
            Example {
                description: "Returns index of pattern in string with start index",
                example: " '.rb.rb' | str index-of '.rb' -r '1,'",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: "Returns index of pattern in string with end index",
                example: " '123456' | str index-of '6' -r ',4'",
                result: Some(Value::test_int(-1)),
            },
            Example {
                description: "Returns index of pattern in string with start and end index",
                example: " '123456' | str index-of '3' -r '1,4'",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Alternatively you can use this form",
                example: " '123456' | str index-of '3' -r [1 4]",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Returns index of pattern in string",
                example: " '/this/is/some/path/file.txt' | str index-of '/' -e",
                result: Some(Value::test_int(18)),
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
    let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;

    let options = Arc::new(Arguments {
        pattern: pattern.item,
        range: call.get_flag(engine_state, stack, "range")?,
        end: call.has_flag("end"),
        column_paths: call.rest(engine_state, stack, 1)?,
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

fn action(
    input: &Value,
    Arguments {
        ref pattern,
        range,
        end,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    let range = match range {
        Some(range) => range.clone(),
        None => Value::String {
            val: "".to_string(),
            span: head,
        },
    };

    let r = process_range(input, &range, head);

    match input {
        Value::String { val: s, .. } => {
            let (start_index, end_index) = match r {
                Ok(r) => (r.0 as usize, r.1 as usize),
                Err(e) => return Value::Error { error: e },
            };

            if *end {
                if let Some(result) = s[start_index..end_index].rfind(&**pattern) {
                    Value::Int {
                        val: result as i64 + start_index as i64,
                        span: head,
                    }
                } else {
                    Value::Int {
                        val: -1,
                        span: head,
                    }
                }
            } else if let Some(result) = s[start_index..end_index].find(&**pattern) {
                Value::Int {
                    val: result as i64 + start_index as i64,
                    span: head,
                }
            } else {
                Value::Int {
                    val: -1,
                    span: head,
                }
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

fn process_range(
    input: &Value,
    range: &Value,
    head: Span,
) -> Result<IndexOfOptionalBounds, ShellError> {
    let input_len = match input {
        Value::String { val: s, .. } => s.len(),
        _ => 0,
    };
    let min_index_str = String::from("0");
    let max_index_str = input_len.to_string();
    let r = match range {
        Value::String { val: s, .. } => {
            let indexes: Vec<&str> = s.split(',').collect();

            let start_index = indexes.get(0).unwrap_or(&&min_index_str[..]).to_string();

            let end_index = indexes.get(1).unwrap_or(&&max_index_str[..]).to_string();

            Ok((start_index, end_index))
        }
        Value::List { vals, .. } => {
            if vals.len() > 2 {
                Err(ShellError::UnsupportedInput(
                    String::from("there shouldn't be more than two indexes. too many indexes"),
                    head,
                ))
            } else {
                let idx: Vec<String> = vals
                    .iter()
                    .map(|v| v.as_string().unwrap_or_else(|_| String::from("")))
                    .collect();

                let start_index = idx.get(0).unwrap_or(&min_index_str).to_string();
                let end_index = idx.get(1).unwrap_or(&max_index_str).to_string();

                Ok((start_index, end_index))
            }
        }
        other => Err(ShellError::UnsupportedInput(
            format!(
                "Input's type is {}. This command only works with strings.",
                other.get_type()
            ),
            head,
        )),
    }?;

    let start_index = r.0.parse::<i32>().unwrap_or(0);
    let end_index = r.1.parse::<i32>().unwrap_or(input_len as i32);

    if start_index < 0 || start_index > end_index {
        return Err(ShellError::UnsupportedInput(
            String::from(
                "start index can't be negative or greater than end index. Invalid start index",
            ),
            head,
        ));
    }

    if end_index < 0 || end_index < start_index || end_index > input_len as i32 {
        return Err(ShellError::UnsupportedInput(
            String::from(
            "end index can't be negative, smaller than start index or greater than input length. Invalid end index"),
            head,
        ));
    }
    Ok(IndexOfOptionalBounds(start_index, end_index))
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
    fn returns_index_of_substring() {
        let word = Value::String {
            val: String::from("Cargo.tomL"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from(".tomL"),

            range: Some(Value::String {
                val: String::from(""),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(5));
    }
    #[test]
    fn index_of_does_not_exist_in_string() {
        let word = Value::String {
            val: String::from("Cargo.tomL"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from("Lm"),

            range: Some(Value::String {
                val: String::from(""),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());

        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_next_substring() {
        let word = Value::String {
            val: String::from("Cargo.Cargo"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from("Cargo"),

            range: Some(Value::String {
                val: String::from("1"),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(6));
    }

    #[test]
    fn index_does_not_exist_due_to_end_index() {
        let word = Value::String {
            val: String::from("Cargo.Banana"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from("Banana"),

            range: Some(Value::String {
                val: String::from(",5"),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }

    #[test]
    fn returns_index_of_nums_in_middle_due_to_index_limit_from_both_ends() {
        let word = Value::String {
            val: String::from("123123123"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from("123"),

            range: Some(Value::String {
                val: String::from("2,6"),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(3));
    }

    #[test]
    fn index_does_not_exists_due_to_strict_bounds() {
        let word = Value::String {
            val: String::from("123456"),
            span: Span::test_data(),
        };

        let options = Arguments {
            pattern: String::from("1"),

            range: Some(Value::String {
                val: String::from("2,4"),
                span: Span::test_data(),
            }),
            column_paths: vec![],
            end: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(-1));
    }
}
