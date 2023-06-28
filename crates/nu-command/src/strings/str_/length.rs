use crate::grapheme_flags;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};
use unicode_segmentation::UnicodeSegmentation;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str length"
    }

    fn signature(&self) -> Signature {
        Signature::build("str length")
            .input_output_types(vec![(Type::String, Type::Int)])
            .vectorizes_over_list(true)
            .switch(
                "grapheme-clusters",
                "count length using grapheme clusters (all visible chars have length 1)",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "count length using UTF-8 bytes (default; all non-ASCII chars have length 2+)",
                Some('b'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, replace strings at the given cell paths with their length",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Output the length of any strings in the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["size", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = Arguments {
            cell_paths: (!cell_paths.is_empty()).then_some(cell_paths),
            graphemes: grapheme_flags(call)?,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the lengths of a string",
                example: "'hello' | str length",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Count length using grapheme clusters",
                example: "'🇯🇵ほげ ふが ぴよ' | str length -g",
                result: Some(Value::test_int(9)),
            },
            Example {
                description: "Return the lengths of multiple strings",
                example: "['hi' 'there'] | str length",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &Value, arg: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::int(
            if arg.graphemes {
                val.graphemes(true).count()
            } else {
                val.len()
            } as i64,
            head,
        ),
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.expect_span(),
            }),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn use_utf8_bytes() {
        let word = Value::String {
            val: String::from("🇯🇵ほげ ふが ぴよ"),
            span: Span::test_data(),
        };

        let options = Arguments {
            cell_paths: None,
            graphemes: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(28));
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
