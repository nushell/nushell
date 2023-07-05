use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct IsAdmin;

impl Command for IsAdmin {
    fn name(&self) -> &str {
        "is-admin"
    }

    fn usage(&self) -> &str {
        "Check if nushell is running with administrator or root privileges."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("is-admin")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::Bool)])
            .allow_variants_without_examples(true)
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
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::boolean(is_root(), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return 'iamroot' if nushell is running with admin/root privileges, and 'iamnotroot' if not.",
                example: r#"if (is-admin) { "iamroot" } else { "iamnotroot" }"#,
                result: Some(Value::test_string("iamnotroot")),
            },
        ]
    }
}

#[cfg(unix)]
fn is_root() -> bool {
    nix::unistd::Uid::current().is_root()
}

#[cfg(windows)]
fn is_root() -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut handle = INVALID_HANDLE_VALUE;
    let mut elevated = false;

    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) != FALSE {
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

            if GetTokenInformation(
                handle,
                TokenElevation,
                Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
                size,
                &mut size,
            ) != FALSE
            {
                elevated = elevation.TokenIsElevated != 0;
            }
        }

        if handle != INVALID_HANDLE_VALUE {
            CloseHandle(handle);
        }
    }

    elevated
}
