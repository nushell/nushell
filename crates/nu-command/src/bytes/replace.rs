use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

struct Arguments {
    find: Vec<u8>,
    replace: Vec<u8>,
    cell_paths: Option<Vec<CellPath>>,
    all: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct BytesReplace;

impl Command for BytesReplace {
    fn name(&self) -> &str {
        "bytes replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes replace")
            .input_output_types(vec![(Type::Binary, Type::Binary)])
            .required("find", SyntaxShape::Binary, "the pattern to find")
            .required("replace", SyntaxShape::Binary, "the replacement pattern")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, replace bytes in data at the given cell paths",
            )
            .switch("all", "replace all occurrences of find binary", Some('a'))
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Find and replace binary"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["search", "shift", "switch"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 2)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let find = call.req::<Spanned<Vec<u8>>>(engine_state, stack, 0)?;
        if find.item.is_empty() {
            return Err(ShellError::UnsupportedInput(
                "the pattern to find cannot be empty".to_string(),
                find.span,
            ));
        }

        let arg = Arguments {
            find: find.item,
            replace: call.req::<Vec<u8>>(engine_state, stack, 1)?,
            cell_paths,
            all: call.has_flag("all"),
        };

        operate(replace, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find and replace contents",
                example: "0x[10 AA FF AA FF] | bytes replace 0x[10 AA] 0x[FF]",
                result: Some(Value::Binary {
                    val: vec![0xFF, 0xFF, 0xAA, 0xFF],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find binary",
                example: "0x[10 AA 10 BB 10] | bytes replace -a 0x[10] 0x[A0]",
                result: Some(Value::Binary {
                    val: vec![0xA0, 0xAA, 0xA0, 0xBB, 0xA0],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find and replace all occurrences of find binary in table",
                example: "[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes replace -a 0x[11] 0x[13] ColA ColC",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::Binary {
                                val: vec![0x13, 0x12, 0x13],
                                span: Span::test_data(),
                            },
                            Value::Binary {
                                val: vec![0x14, 0x15, 0x16],
                                span: Span::test_data(),
                            },
                            Value::Binary {
                                val: vec![0x17, 0x18, 0x19],
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn replace(val: &Value, args: &Arguments, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => replace_impl(val, args, *val_span),
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with bytes.",
                    other.get_type()
                ),
                span,
            ),
        },
    }
}

fn replace_impl(input: &[u8], arg: &Arguments, span: Span) -> Value {
    let mut replaced = vec![];
    let replace_all = arg.all;

    // doing find-and-replace stuff.
    let (mut left, mut right) = (0, arg.find.len());
    let input_len = input.len();
    let pattern_len = arg.find.len();
    while right <= input_len {
        if input[left..right] == arg.find {
            let mut to_replace = arg.replace.clone();
            replaced.append(&mut to_replace);
            left += pattern_len;
            right += pattern_len;
            if !replace_all {
                break;
            }
        } else {
            replaced.push(input[left]);
            left += 1;
            right += 1;
        }
    }

    let mut remain = input[left..].to_vec();
    replaced.append(&mut remain);
    Value::Binary {
        val: replaced,
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesReplace {})
    }
}
