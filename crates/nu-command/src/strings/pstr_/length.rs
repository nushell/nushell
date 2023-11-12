use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};
use print_positions::print_positions;

struct Arguments {
    cell_paths: Option<Vec<CellPath>>,
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
        "pstr length"
    }

    fn signature(&self) -> Signature {
        Signature::build("pstr length")
            .input_output_types(vec![
                (Type::String, Type::Int),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Int))),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, replace strings at the given cell paths with their length",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Output the rendered length of a string, counting extended grapheme cluster as one \"print position\" and skipping ANSI control sequences."
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
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the length of a string",
                example: "'hello' | pstr length",
                result: Some(Value::test_int(5)),
            },
            Example {
                description:
                    "Return the rendered length of a string ignoring ANSI control sequences",
                example: "$\"plain(ansi cyan)cyan(ansi reset)\" | pstr length",
                result: Some(Value::test_int(9)),
            },
            Example {
                description: "Return the rendered length of a string counting a grapheme cluster as one \"print position\"",
                example: "'こんにちは世界' | pstr length",
                result: Some(Value::test_int(7)),
            },
        ]
    }
}

fn action(input: &Value, _arg: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::int(print_positions(val).count() as i64, head),
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
