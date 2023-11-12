use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_cmd_base::util;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{
    Example, PipelineData, Range, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use print_positions::print_position_data;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    indexes: Substring,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
struct Substring(isize, isize);

impl From<(isize, isize)> for Substring {
    fn from(input: (isize, isize)) -> Substring {
        Substring(input.0, input.1)
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "pstr substring"
    }

    fn signature(&self) -> Signature {
        Signature::build("pstr substring")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .required(
                "range",
                SyntaxShape::Any,
                "the indexes to substring [start end]",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, turn strings at the given cell paths into substrings",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Get part of a string, indexing by \"print positions\". Note that the start is included but the end is excluded."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["slice"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let range: Range = call.req(engine_state, stack, 0)?;

        let indexes = match util::process_range(&range) {
            Ok(idxs) => idxs.into(),
            Err(processing_error) => {
                return Err(processing_error("could not perform substring", call.head))
            }
        };

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            indexes,
            cell_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Extract substring from colorized string, counting just print positions (skips ANSI control sequences)",
                example: r#"let s = ($"plain(ansi cyan)cyan(ansi red)red(ansi reset)")
    [ $s,
      ($s | pstr substring 5..12)
    ] | str join "\n""#,
                result: Some(
                        Value::test_string("plain\u{1b}[36mcyan\u{1b}[31mred\u{1b}[0m\n\u{1b}[36mcyan\u{1b}[31mred\u{1b}[0m"),
                )
            },
            Example {
                description: "Extract substring from UTF-8 string containing multibyte characters (counts extended grapheme cluster as 1 print position)",
                example: " 'こんにちは世界' | pstr substring 5..7",
                result: Some(Value::test_string("世界")),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let options = &args.indexes;
    match input {
        Value::String { val: s, .. } => {
            let len: isize = s.len() as isize;

            let start: isize = if options.0 < 0 {
                options.0 + len
            } else {
                options.0
            };
            let end: isize = if options.1 < 0 {
                std::cmp::max(len + options.1, 0)
            } else {
                options.1
            };

            if start < len && end >= 0 {
                match start.cmp(&end) {
                    Ordering::Equal => Value::string("", head),
                    Ordering::Greater => Value::error(
                        ShellError::TypeMismatch {
                            err_message: "End must be greater than or equal to Start".to_string(),
                            span: head,
                        },
                        head,
                    ),
                    Ordering::Less => Value::string(
                        if end == isize::max_value() {
                            print_position_data(s)
                                .skip(start as usize)
                                .collect::<Vec<&str>>()
                                .join("")
                        } else {
                            print_position_data(s)
                                .skip(start as usize)
                                .take((end - start) as usize)
                                .collect::<Vec<&str>>()
                                .join("")
                        },
                        head,
                    ),
                }
            } else {
                Value::string("", head)
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::UnsupportedInput {
                msg: "Only string values are supported".into(),
                input: format!("input type: {:?}", other.get_type()),
                msg_span: head,
                input_span: other.span(),
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
