use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Hide;

impl Command for Hide {
    fn name(&self) -> &str {
        "hide"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("hide")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("pattern", SyntaxShape::ImportPattern, "import pattern")
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Hide definitions in the current scope"
    }

    fn extra_usage(&self) -> &str {
        r#"Definitions are hidden by priority: First aliases, then custom commands.

This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let env_var_name = if let Some(Expression {
            expr: Expr::ImportPattern(pat),
            ..
        }) = call.positional_nth(0)
        {
            Spanned {
                item: String::from_utf8_lossy(&pat.head.name).to_string(),
                span: pat.head.span,
            }
        } else {
            return Err(ShellError::GenericError(
                "Unexpected import".into(),
                "import pattern not supported".into(),
                Some(call.head),
                None,
                Vec::new(),
            ));
        };

        stack.remove_env_var(engine_state, &env_var_name.item);

        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Hide the alias just defined",
                example: r#"alias lll = ls -l; hide lll"#,
                result: None,
            },
            Example {
                description: "Hide a custom command",
                example: r#"def say-hi [] { echo 'Hi!' }; hide say-hi"#,
                result: None,
            },
            Example {
                description: "Hide an environment variable",
                example: r#"let-env HZ_ENV_ABC = 1; hide HZ_ENV_ABC; 'HZ_ENV_ABC' in (env).name"#,
                result: Some(Value::boolean(false, Span::test_data())),
            },
        ]
    }
}
