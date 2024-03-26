use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Def;

impl Command for Def {
    fn name(&self) -> &str {
        "def"
    }

    fn usage(&self) -> &str {
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

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
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
                description: "Define a custom wrapper for an external command",
                example: r#"def --wrapped my-echo [...rest] { echo $rest }; my-echo spam"#,
                result: Some(Value::test_list(vec![Value::test_string("spam")])),
            },
        ]
    }
}
