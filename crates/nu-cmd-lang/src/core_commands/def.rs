use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Def;

impl Command for Def {
    fn name(&self) -> &str {
        "def"
    }

    fn description(&self) -> &str {
        "Define a custom command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("def")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("def_name", SyntaxShape::String, "Command name.")
            .required("params", SyntaxShape::Signature, "Parameters.")
            .required("block", SyntaxShape::Closure(None), "Body of the definition.")
            .switch("env", "keep the environment defined inside the command", None)
            .switch("wrapped", "treat unknown flags and arguments as strings (requires ...rest-like parameter in signature)", None)
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
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Define a command and run it",
                example: r#"def say-hi [] { echo 'hi' }; say-hi"#,
                result: Some(Value::test_string("hi")),
            },
            Example {
                description: "Define a command and run it with parameter(s)",
                example: r#"def say-sth [sth: string] { echo $sth }; say-sth hi"#,
                result: Some(Value::test_string("hi")),
            },
            Example {
                description: "Set environment variable by call a custom command",
                example: r#"def --env foo [] { $env.BAR = "BAZ" }; foo; $env.BAR"#,
                result: Some(Value::test_string("BAZ")),
            },
            Example {
                description: "cd affects the environment, so '--env' is required to change directory from within a command",
                example: r#"def --env gohome [] { cd ~ }; gohome; $env.PWD == ('~' | path expand)"#,
                result: Some(Value::test_string("true")),
            },
            Example {
                description: "Define a custom wrapper for an external command",
                example: r#"def --wrapped my-echo [...rest] { ^echo ...$rest }; my-echo -e 'spam\tspam'"#,
                result: Some(Value::test_string("spam\tspam")),
            },
            Example {
                description: "Define a custom command with a type signature. Passing a non-int value will result in an error",
                example: r#"def only_int []: int -> int { $in }; 42 | only_int"#,
                result: Some(Value::test_int(42)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_parser::parse;

    fn get_test_env() -> EngineState {
        let mut engine_state = EngineState::new();
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            working_set.add_decl(Box::new(Def));
            working_set.render()
        };
        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");
        engine_state
    }
    #[test]
    fn test_did_you_mean() {
        let engine_state = get_test_env();
        let script = b"
def foo [--a-long-name] {
  $a-long-name
}
";
        let mut working_set = StateWorkingSet::new(&engine_state);

        parse(&mut working_set, None, script, true);
        assert_eq!(
            format!("{:?}", working_set.parse_errors),
            "[VariableNotFound(DidYouMean(Some(\"$a_long_name\")), Span { start: 29, end: 41 }), Expected(\"valid variable name\", Span { start: 29, end: 41 })]"
        );
    }
}
