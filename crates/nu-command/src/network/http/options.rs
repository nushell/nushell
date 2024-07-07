use crate::network::http::client::{
    http_client, http_parse_url, request_add_authorization_header, request_add_custom_headers,
    request_handle_response, request_set_timeout, send_request, RedirectMode, RequestFlags,
};
use nu_engine::command_prelude::*;

use super::client::HttpBody;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "http options"
    }

    fn signature(&self) -> Signature {
        Signature::build("http options")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .required(
                "URL",
                SyntaxShape::String,
                "The URL to fetch the options from.",
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
                "insecure",
                "allow insecure server connections when using SSL",
                Some('k'),
            )
            .switch(
                "allow-errors",
                "do not fail if the server returns an error code",
                Some('e'),
            )
            .filter()
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Requests permitted communication options for a given URL."
    }

    fn extra_usage(&self) -> &str {
        "Performs an HTTP OPTIONS request. Most commonly used for making CORS preflight requests."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "fetch", "pull", "request", "curl", "wget"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_get(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get options from example.com",
                example: "http options https://www.example.com",
                result: None,
            },
            Example {
                description: "Get options from example.com, with username and password",
                example: "http options --user myuser --password mypass https://www.example.com",
                result: None,
            },
            Example {
                description: "Get options from example.com, with custom header",
                example: "http options --headers [my-header-key my-header-value] https://www.example.com",
                result: None,
            },
            Example {
                description: "Get options from example.com, with custom headers",
                example: "http options --headers [my-header-key-A my-header-value-A my-header-key-B my-header-value-B] https://www.example.com",
                result: None,
            },
            Example {
                description: "Simulate a browser cross-origin preflight request from www.example.com to media.example.com",
                example: "http options https://media.example.com/api/ --headers [Origin https://www.example.com Access-Control-Request-Headers \"Content-Type, X-Custom-Header\" Access-Control-Request-Method GET]",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    headers: Option<Value>,
    insecure: bool,
    user: Option<String>,
    password: Option<String>,
    timeout: Option<Value>,
    allow_errors: bool,
}

fn run_get(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        url: call.req(engine_state, stack, 0)?,
        headers: call.get_flag(engine_state, stack, "headers")?,
        insecure: call.has_flag(engine_state, stack, "insecure")?,
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        timeout: call.get_flag(engine_state, stack, "max-time")?,
        allow_errors: call.has_flag(engine_state, stack, "allow-errors")?,
    };
    helper(engine_state, stack, call, args)
}

// Helper function that actually goes to retrieve the resource from the url given
// The Option<String> return a possible file extension which can be used in AutoConvert commands
fn helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    args: Arguments,
) -> Result<PipelineData, ShellError> {
    let span = args.url.span();
    let ctrl_c = engine_state.ctrlc.clone();
    let (requested_url, _) = http_parse_url(call, span, args.url)?;

    let client = http_client(args.insecure, RedirectMode::Follow, engine_state, stack)?;
    let mut request = client.request("OPTIONS", &requested_url);

    request = request_set_timeout(args.timeout, request)?;
    request = request_add_authorization_header(args.user, args.password, request);
    request = request_add_custom_headers(args.headers, request)?;

    let response = send_request(request.clone(), HttpBody::None, None, ctrl_c);

    // http options' response always showed in header, so we set full to true.
    // And `raw` is useless too because options method doesn't return body, here we set to true
    // too.
    let request_flags = RequestFlags {
        raw: true,
        full: true,
        allow_errors: args.allow_errors,
    };

    request_handle_response(
        engine_state,
        stack,
        span,
        &requested_url,
        request_flags,
        response,
        request,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
