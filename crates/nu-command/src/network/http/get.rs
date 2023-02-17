use crate::network::http::client::{
    http_client, request_add_authorization_header, request_add_custom_headers,
    request_handle_response, request_set_timeout,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "http get"
    }

    fn signature(&self) -> Signature {
        Signature::build("http get")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required(
                "URL",
                SyntaxShape::String,
                "the URL to fetch the contents from",
            )
            .named(
                "user",
                SyntaxShape::Any,
                "the username when authenticating",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
                Some('p'),
            )
            .named(
                "max-time",
                SyntaxShape::Int,
                "timeout period in seconds",
                Some('m'),
            )
            .named(
                "headers",
                SyntaxShape::Any,
                "custom headers you want to add ",
                Some('H'),
            )
            .switch(
                "raw",
                "fetch contents as text rather than a table",
                Some('r'),
            )
            .switch(
                "insecure",
                "allow insecure server connections when using SSL",
                Some('k'),
            )
            .filter()
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Fetch the contents from a URL."
    }

    fn extra_usage(&self) -> &str {
        "Performs HTTP GET operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "network", "fetch", "pull", "request", "download", "curl", "wget",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_post(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "http get content from example.com",
                example: "http get https://www.example.com",
                result: None,
            },
            Example {
                description: "http get content from example.com, with username and password",
                example: "http get -u myuser -p mypass https://www.example.com",
                result: None,
            },
            Example {
                description: "http get content from example.com, with custom header",
                example: "http get -H [my-header-key my-header-value] https://www.example.com",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    raw: bool,
    insecure: Option<bool>,
    user: Option<String>,
    password: Option<String>,
    timeout: Option<Value>,
    headers: Option<Value>,
}

fn run_post(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        url: call.req(engine_state, stack, 0)?,
        raw: call.has_flag("raw"),
        insecure: call.get_flag(engine_state, stack, "insecure")?,
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        timeout: call.get_flag(engine_state, stack, "timeout")?,
        headers: call.get_flag(engine_state, stack, "headers")?,
    };
    helper(engine_state, stack, args)
}

// Helper function that actually goes to retrieve the resource from the url given
// The Option<String> return a possible file extension which can be used in AutoConvert commands
fn helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let url_value = args.url;
    let user = args.user.clone();
    let password = args.password;
    let timeout = args.timeout;
    let headers = args.headers;
    let raw = args.raw;

    let span = url_value.span()?;
    let requested_url = url_value.as_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::TypeMismatch(
                "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(),
                span,
            ));
        }
    };

    let client = http_client(args.insecure.is_some());
    let mut request = client.get(url);

    request = request_set_timeout(timeout, request)?;
    request = request_add_authorization_header(user, password, request);
    request = request_add_custom_headers(headers, request)?;

    let response = request.send().and_then(|r| r.error_for_status());
    request_handle_response(engine_state, stack, span, &requested_url, raw, response)
}
