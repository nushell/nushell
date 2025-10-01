use crate::{grapheme_flags, grapheme_flags_const};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use unicode_segmentation::UnicodeSegmentation;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
    graphemes: bool,
    chars: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct StrLength;

impl Command for StrLength {
    fn name(&self) -> &str {
        "str length"
    }

    fn signature(&self) -> Signature {
        Signature::build("str length")
            .input_output_types(vec![
                (Type::String, Type::Int),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Int))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "grapheme-clusters",
                "count length in grapheme clusters (all visible chars have length 1)",
                Some('g'),
            )
            .switch(
                "utf-8-bytes",
                "count length in UTF-8 bytes (default; all non-ASCII chars have length 2+)",
                Some('b'),
            )
            .switch(
                "chars",
                "count length in chars",
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, replace strings at the given cell paths with their length.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Output the length of any strings in the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["size", "count"]
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
        let chars = call.has_flag(engine_state, stack, "chars")?;
        run(
            cell_paths,
            engine_state,
            call,
            input,
            grapheme_flags(engine_state, stack, call)?,
            chars,
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
        let chars = call.has_flag_const(working_set, "chars")?;
        run(
            cell_paths,
            working_set.permanent(),
            call,
            input,
            grapheme_flags_const(working_set, call)?,
            chars,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return the lengths of a string in bytes",
                example: "'hello' | str length",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Count length of a string in grapheme clusters",
                example: "'🇯🇵ほげ ふが ぴよ' | str length  --grapheme-clusters",
                result: Some(Value::test_int(9)),
            },
            Example {
                description: "Return the lengths of multiple strings in bytes",
                example: "['hi' 'there'] | str length",
                result: Some(Value::list(
                    vec![Value::test_int(2), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Return the lengths of a string in chars",
                example: "'hällo' | str length --chars",
                result: Some(Value::test_int(5)),
            },
        ]
    }
}

fn run(
    cell_paths: Vec<CellPath>,
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
    graphemes: bool,
    chars: bool,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        cell_paths: (!cell_paths.is_empty()).then_some(cell_paths),
        graphemes,
        chars,
    };
    operate(action, args, input, call.head, engine_state.signals())
}

fn action(input: &Value, arg: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::int(
            if arg.graphemes {
                val.graphemes(true).count()
            } else if arg.chars {
                val.chars().count()
            } else {
                val.len()
            } as i64,
            head,
        ),
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
mod test {
    use super::*;

    #[test]
    fn use_utf8_bytes() {
        let word = Value::string(String::from("🇯🇵ほげ ふが ぴよ"), Span::test_data());

        let options = Arguments {
            cell_paths: None,
            graphemes: false,
            chars: false,
        };

        let actual = action(&word, &options, Span::test_data());
        assert_eq!(actual, Value::test_int(28));
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrLength {})
    }
}
