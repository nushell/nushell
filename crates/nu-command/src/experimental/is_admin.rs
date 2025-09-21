use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IsAdmin;

impl Command for IsAdmin {
    fn name(&self) -> &str {
        "is-admin"
    }

    fn description(&self) -> &str {
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
        Ok(Value::bool(is_root(), call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Return 'iamroot' if nushell is running with admin/root privileges, and 'iamnotroot' if not.",
            example: r#"if (is-admin) { "iamroot" } else { "iamnotroot" }"#,
            result: Some(Value::test_string("iamnotroot")),
        }]
    }
}

/// Returns `true` if user is root; `false` otherwise
fn is_root() -> bool {
    is_root_impl()
}

#[cfg(unix)]
fn is_root_impl() -> bool {
    nix::unistd::Uid::current().is_root()
}

#[cfg(windows)]
fn is_root_impl() -> bool {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut elevated = false;

    // Checks whether the access token associated with the current process has elevated privileges.
    // SAFETY: `elevated` only touched by safe code.
    // `handle` lives long enough, initialized, mutated, used and closed with validity check.
    // `elevation` only read on success and passed with correct `size`.
    unsafe {
        let mut handle = HANDLE::default();

        // Opens the access token associated with the current process.
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

            // Retrieves elevation token information about the access token associated with the current process.
            // Call available since XP
            // https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation
            if GetTokenInformation(
                handle,
                TokenElevation,
                Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
                size,
                &mut size,
            )
            .is_ok()
            {
                // Whether the token has elevated privileges.
                // Safe to read as `GetTokenInformation` will not write outside `elevation` and it succeeded
                // See: https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation#parameters
                elevated = elevation.TokenIsElevated != 0;
            }
        }

        if !handle.is_invalid() {
            // Closes the object handle.
            let _ = CloseHandle(handle);
        }
    }

    elevated
}

#[cfg(target_arch = "wasm32")]
fn is_root_impl() -> bool {
    // in wasm we don't have a user system, so technically we are never root
    false
}
