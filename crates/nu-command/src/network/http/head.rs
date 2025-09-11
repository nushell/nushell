use crate::network::http::client::{
    check_response_redirection, extract_response_headers, handle_response_status, headers_to_nu,
    http_client, http_parse_redirect_mode, http_parse_url, request_add_authorization_header,
    request_add_custom_headers, request_set_timeout, send_request_no_body,
};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;

use super::client::RedirectMode;

#[derive(Clone)]
pub struct HttpHead;

impl Command for HttpHead {
    fn name(&self) -> &str {
        "http head"
    }

    fn signature(&self) -> Signature {
        Signature::build("http head")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .required(
                "URL",
                SyntaxShape::String,
                "The URL to fetch the contents from.",
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
                SyntaxShape::Duration,
                "max duration before timeout occurs",
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
            .param(
                Flag::new("redirect-mode")
                    .short('R')
                    .arg(SyntaxShape::String)
                    .desc(
                        "What to do when encountering redirects. Default: 'follow'. Valid \
                         options: 'follow' ('f'), 'manual' ('m'), 'error' ('e').",
                    )
                    .completion(Completion::new_list(RedirectMode::MODES)),
            )
            .filter()
            .category(Category::Network)
    }

    fn description(&self) -> &str {
        "Get the headers from a URL."
    }

    fn extra_description(&self) -> &str {
        "Performs HTTP HEAD operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "request", "curl", "wget", "headers", "header"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_head(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get headers from example.com",
                example: "http head https://www.example.com",
                result: None,
            },
            Example {
                description: "Get headers from example.com, with username and password",
                example: "http head --user myuser --password mypass https://www.example.com",
                result: None,
            },
            Example {
                description: "Get headers from example.com, with custom header using a record",
                example: "http head --headers {my-header-key: my-header-value} https://www.example.com",
                result: None,
            },
            Example {
                description: "Get headers from example.com, with custom header using a list",
                example: "http head --headers [my-header-key-A my-header-value-A my-header-key-B my-header-value-B] https://www.example.com",
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
    redirect: Option<Spanned<String>>,
}

fn run_head(
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
        redirect: call.get_flag(engine_state, stack, "redirect-mode")?,
    };

    helper(engine_state, stack, call, args, engine_state.signals())
}

// Helper function that actually goes to retrieve the resource from the url given
fn helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    args: Arguments,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let span = args.url.span();
    let (requested_url, _) = http_parse_url(call, span, args.url)?;
    let redirect_mode = http_parse_redirect_mode(args.redirect)?;

    let client = http_client(args.insecure, redirect_mode, engine_state, stack)?;
    let mut request = client.head(&requested_url);

    request = request_set_timeout(args.timeout, request)?;
    request = request_add_authorization_header(args.user, args.password, request);
    request = request_add_custom_headers(args.headers, request)?;

    let (response, _request_headers) = send_request_no_body(request, call.head, signals);
    let response = response?;
    check_response_redirection(redirect_mode, span, &response)?;
    handle_response_status(&response, redirect_mode, &requested_url, span, false)?;
    headers_to_nu(&extract_response_headers(&response), span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(HttpHead {})
    }
}
