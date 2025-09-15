use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Match;

impl Command for Match {
    fn name(&self) -> &str {
        "match"
    }

    fn description(&self) -> &str {
        "Conditionally run a block on a matched value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("match")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("value", SyntaxShape::Any, "Value to check.")
            .required(
                "match_block",
                SyntaxShape::MatchBlock,
                "Block to run if check succeeds.",
            )
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
            "Tried to execute 'run' for the 'match' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Match on a value",
                example: "match 3 { 1 => 'one', 2 => 'two', 3 => 'three' }",
                result: Some(Value::test_string("three")),
            },
            Example {
                description: "Match against alternative values",
                example: "match 'three' { 1 | 'one' => '-', 2 | 'two' => '--', 3 | 'three' => '---' }",
                result: Some(Value::test_string("---")),
            },
            Example {
                description: "Match on a value in range",
                example: "match 3 { 1..10 => 'yes!' }",
                result: Some(Value::test_string("yes!")),
            },
            Example {
                description: "Match on a field in a record",
                example: "match {a: 100} { {a: $my_value} => { $my_value } }",
                result: Some(Value::test_int(100)),
            },
            Example {
                description: "Match with a catch-all",
                example: "match 3 { 1 => { 'yes!' }, _ => { 'no!' } }",
                result: Some(Value::test_string("no!")),
            },
            Example {
                description: "Match against a list",
                example: "match [1, 2, 3] { [$a, $b, $c] => { $a + $b + $c }, _ => 0 }",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Match against pipeline input",
                example: "{a: {b: 3}} | match $in {{a: { $b }} => ($b + 10) }",
                result: Some(Value::test_int(13)),
            },
            Example {
                description: "Match with a guard",
                example: "match [1 2 3] {
        [$x, ..$y] if $x == 1 => { 'good list' },
        _ => { 'not a very good list' }
    }
    ",
                result: Some(Value::test_string("good list")),
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

        test_examples(Match {})
    }
}
