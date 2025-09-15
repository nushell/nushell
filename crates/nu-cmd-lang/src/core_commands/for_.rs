use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct For;

impl Command for For {
    fn name(&self) -> &str {
        "for"
    }

    fn description(&self) -> &str {
        "Loop over a range."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("for")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required(
                "var_name",
                SyntaxShape::VarWithOptType,
                "Name of the looping variable.",
            )
            .required(
                "range",
                SyntaxShape::Keyword(b"in".to_vec(), Box::new(SyntaxShape::Any)),
                "Range of the loop.",
            )
            .required("block", SyntaxShape::Block, "The block to run.")
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        eprintln!(
            "Tried to execute 'run' for the 'for' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Print the square of each integer",
                example: "for x in [1 2 3] { print ($x * $x) }",
                result: None,
            },
            Example {
                description: "Work with elements of a range",
                example: "for $x in 1..3 { print $x }",
                result: None,
            },
            Example {
                description: "Number each item and print a message",
                example: r#"for $it in (['bob' 'fred'] | enumerate) { print $"($it.index) is ($it.item)" }"#,
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(For {})
    }
}
