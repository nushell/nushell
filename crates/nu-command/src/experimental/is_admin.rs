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
        Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    let mut handle = INVALID_HANDLE_VALUE;
    let mut elevated = false;

    unsafe {
        // Opens the access token associated with a process.
        //
        // [Reference](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.OpenProcessToken.html)
        // [Further reading](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocesstoken)
        //
        // # Signature
        // unsafe fn OpenProcessToken<P0>(
        //     processhandle: P0,
        //     desiredaccess: TOKEN_ACCESS_MASK,
        //     tokenhandle: *mut HANDLE
        // ) -> BOOL where P0: IntoParam<HANDLE>
        //
        // # Parameters
        // processhandle: A handle to the process whose access token is opened.
        // desiredaccess: Specifies an access mask that specifies the requested types of access to the access token.
        // tokenhandle: A pointer to a handle that identifies the newly opened access token when the function returns.
        //
        // # Return value
        // If the function succeeds, the return value is nonzero.
        // If the function fails, the return value is zero.
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) != FALSE {
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

            // Retrieves a specified type of information about an access token.
            //
            // [Reference](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/fn.GetTokenInformation.html)
            // [Further reading](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation)
            //
            // # Signature
            // unsafe fn GetTokenInformation<P0>(
            //     tokenhandle: P0,
            //     tokeninformationclass: TOKEN_INFORMATION_CLASS,
            //     tokeninformation: Option<*mut c_void>,
            //     tokeninformationlength: u32,
            //     returnlength: *mut u32
            // ) -> BOOL where P0: IntoParam<HANDLE>
            //
            // # Parameters
            // tokenhandle: A handle to an access token from which information is retrieved.
            // tokeninformationclass: Specifies a value from the TOKEN_INFORMATION_CLASS enumerated type to identify the type of information the function retrieves.
            // tokeninformation: Contains a pointer to a buffer the function fills with the requested information or `None`.
            // tokeninformationlength: Specifies the size, in bytes, of the buffer pointed to by the TokenInformation parameter. If TokenInformation is `None`, this parameter must be zero.
            // returnlength: A pointer to a variable that receives the number of bytes needed for the buffer pointed to by the TokenInformation parameter. If this value is larger than the value specified in the TokenInformationLength parameter, the function fails and stores no data in the buffer.
            //
            // # Return value
            // If the function succeeds, the return value is nonzero.
            // If the function fails, the return value is zero.
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
            // Closes an open object handle.
            //
            // [Reference](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Foundation/fn.CloseHandle.html)
            // [Further reading](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle)
            //
            // # Signature
            // unsafe fn CloseHandle<P0>(hobject: P0) -> BOOL
            //
            // # Parameters
            // hobject: A valid handle to an open object.
            //
            // # Return value
            // If the function succeeds, the return value is nonzero.
            // If the function fails, the return value is zero.
            CloseHandle(handle);
        }
    }

    elevated
}
