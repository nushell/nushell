use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type, Value,
};

use crate::network::http::client::{
    check_response_redirection, http_client, http_parse_redirect_mode, http_parse_url,
    request_add_authorization_header, request_add_custom_headers, request_handle_response,
    request_set_timeout, send_request,
};

use super::client::RequestFlags;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "http delete"
    }

    fn signature(&self) -> Signature {
        Signature::build("http delete")
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
            .named("data", SyntaxShape::Any, "the content to post", Some('d'))
            .named(
                "content-type",
                SyntaxShape::Any,
                "the MIME type of content to post",
                Some('t'),
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
            .switch(
                "full",
                "returns the full response instead of only the body",
                Some('f'),
            )
            .switch(
                "allow-errors",
                "do not fail if the server returns an error code",
                Some('e'),
            ).named(
                "redirect-mode",
                SyntaxShape::String,
                "What to do when encountering redirects. Default: 'follow'. Valid options: 'follow' ('f'), 'manual' ('m'), 'error' ('e').",
                Some('R')
            )
            .filter()
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Delete the specified resource."
    }

    fn extra_usage(&self) -> &str {
        "Performs HTTP DELETE operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "request", "curl", "wget"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_delete(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "http delete from example.com",
                example: "http delete https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with username and password",
                example: "http delete --user myuser --password mypass https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with custom header",
                example: "http delete --headers [my-header-key my-header-value] https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with body",
                example: "http delete --data 'body' https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with JSON body",
                example:
                    "http delete --content-type application/json --data { field: value } https://www.example.com",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    headers: Option<Value>,
    data: Option<Value>,
    content_type: Option<String>,
    raw: bool,
    insecure: bool,
    user: Option<String>,
    password: Option<String>,
    timeout: Option<Value>,
    full: bool,
    allow_errors: bool,
    redirect: Option<Spanned<String>>,
}

fn run_delete(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        url: call.req(engine_state, stack, 0)?,
        headers: call.get_flag(engine_state, stack, "headers")?,
        data: call.get_flag(engine_state, stack, "data")?,
        content_type: call.get_flag(engine_state, stack, "content-type")?,
        raw: call.has_flag("raw"),
        insecure: call.has_flag("insecure"),
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        timeout: call.get_flag(engine_state, stack, "max-time")?,
        full: call.has_flag("full"),
        allow_errors: call.has_flag("allow-errors"),
        redirect: call.get_flag(engine_state, stack, "redirect-mode")?,
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
    let redirect_mode = http_parse_redirect_mode(args.redirect)?;

    let client = http_client(args.insecure, redirect_mode, engine_state, stack);
    let mut request = client.delete(&requested_url);

    request = request_set_timeout(args.timeout, request)?;
    request = request_add_authorization_header(args.user, args.password, request);
    request = request_add_custom_headers(args.headers, request)?;

    let response = send_request(request.clone(), args.data, args.content_type, ctrl_c);

    let request_flags = RequestFlags {
        raw: args.raw,
        full: args.full,
        allow_errors: args.allow_errors,
    };

    check_response_redirection(redirect_mode, span, &response)?;
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
