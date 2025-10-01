use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use nu_utils::IgnoreCaseExt;

struct Arguments {
    substring: String,
    cell_paths: Option<Vec<CellPath>>,
    case_insensitive: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]

pub struct StrStartsWith;

impl Command for StrStartsWith {
    fn name(&self) -> &str {
        "str starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str starts-with")
            .input_output_types(vec![
                (Type::String, Type::Bool),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Bool))),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("string", SyntaxShape::String, "The string to match.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result.",
            )
            .switch("ignore-case", "search is case insensitive", Some('i'))
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Check if an input starts with a string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prefix", "match", "find", "search"]
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
        let substring: Spanned<String> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            cell_paths,
            case_insensitive: call.has_flag(engine_state, stack, "ignore-case")?,
        };
        operate(action, args, input, call.head, engine_state.signals())
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req_const(working_set, 0)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: substring.item,
            cell_paths,
            case_insensitive: call.has_flag_const(working_set, "ignore-case")?,
        };
        operate(
            action,
            args,
            input,
            call.head,
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Checks if input string starts with 'my'",
                example: "'my_library.rb' | str starts-with 'my'",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if input string starts with 'Car'",
                example: "'Cargo.toml' | str starts-with 'Car'",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if input string starts with '.toml'",
                example: "'Cargo.toml' | str starts-with '.toml'",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Checks if input string starts with 'cargo', case-insensitive",
                example: "'Cargo.toml' | str starts-with --ignore-case 'cargo'",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}

fn action(
    input: &Value,
    Arguments {
        substring,
        case_insensitive,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let starts_with = if *case_insensitive {
                s.to_folded_case().starts_with(&substring.to_folded_case())
            } else {
                s.starts_with(substring)
            };
            Value::bool(starts_with, head)
        }
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
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StrStartsWith {})
    }
}
