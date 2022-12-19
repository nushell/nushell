use is_root::is_root;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Span, Value};

#[derive(Clone)]
pub struct IsAdmin;

impl Command for IsAdmin {
    fn name(&self) -> &str {
        "is-admin"
    }

    fn usage(&self) -> &str {
        "Check if nushell is running with administrator or root privileges"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("is-admin").category(Category::Core)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["root", "administrator", "superuser", "supervisor"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::boolean(is_root(), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return 'iamroot' if nushell is running with admin/root privileges, and 'iamnotroot' if not.",
                example: r#"if is-admin { "iamroot" } else { "iamnotroot" }"#,
                result: Some(Value::string("iamnotroot", Span::test_data())),
            },
        ]
    }
}
