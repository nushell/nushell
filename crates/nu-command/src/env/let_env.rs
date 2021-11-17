use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

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
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::String)),
                "equals sign followed by value",
            )
            .category(Category::Env)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let env_var = call.positional[0]
            .as_string()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");

        let rhs = eval_expression(engine_state, stack, keyword_expr)?;
        let rhs = rhs.as_string()?;

        //println!("Adding: {:?} to {}", rhs, var_id);

        stack.add_env_var(env_var, rhs);
        Ok(PipelineData::new(call.head))
    }
}
