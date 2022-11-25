use nu_engine::{eval_expression_with_input, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LetEnv;

impl Command for LetEnv {
    fn name(&self) -> &str {
        "let-env"
    }

    fn usage(&self) -> &str {
        "Create an environment variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let-env")
            .required("var_name", SyntaxShape::String, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            )
            .category(Category::Env)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        // TODO: find and require the crossplatform restrictions on environment names
        let env_var: Spanned<String> = call.req(engine_state, stack, 0)?;

        let keyword_expr = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_keyword()
            .expect("internal error: missing keyword");

        let rhs =
            eval_expression_with_input(engine_state, stack, keyword_expr, input, false, true)?
                .0
                .into_value(call.head);

        if env_var.item == "FILE_PWD" || env_var.item == "PWD" {
            return Err(ShellError::AutomaticEnvVarSetManually(
                env_var.item,
                env_var.span,
            ));
        } else {
            stack.add_env_var(env_var.item, rhs);
        }
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create an environment variable and display it",
            example: "let-env MY_ENV_VAR = 1; $env.MY_ENV_VAR",
            result: Some(Value::test_int(1)),
        }]
    }
}
