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
        Ok(Value::Bool {
            val: is_root(),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Echo 'iamroot' if nushell is running with admin/root privileges, and 'iamnotroot' if not.",
                example: r#"if is-admin { echo "iamroot" } else { echo "iamnotroot" }"#,
                result: Some(Value::String {
                    val: "iamnotroot".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}
