use crate::network::http::client::{
    HttpBody, RequestFlags, RequestMetadata, check_response_redirection, http_client,
    http_parse_redirect_mode, http_parse_url, request_add_authorization_header,
    request_add_custom_headers, request_handle_response, request_set_timeout, send_request,
    send_request_no_body,
};
use nu_engine::command_prelude::*;

use super::client::RedirectMode;

#[derive(Clone)]
pub struct HttpDelete;

impl Command for HttpDelete {
    fn name(&self) -> &str {
        "http delete"
    }

    fn signature(&self) -> Signature {
        Signature::build("http delete")
            .input_output_types(vec![(Type::Any, Type::Any)])
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
        "Delete the specified resource."
    }

    fn extra_description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
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
                description: "http delete from example.com, with custom header using a record",
                example: "http delete --headers {my-header-key: my-header-value} https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with custom header using a list",
                example: "http delete --headers [my-header-key-A my-header-value-A my-header-key-B my-header-value-B] https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with body",
                example: "http delete --data 'body' https://www.example.com",
                result: None,
            },
            Example {
                description: "http delete from example.com, with JSON body",
                example: "http delete --content-type application/json --data { field: value } https://www.example.com",
                result: None,
            },
            Example {
                description: "Perform an HTTP delete with JSON content from a pipeline to example.com",
                example: "open foo.json | http delete https://www.example.com",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    headers: Option<Value>,
    data: Option<HttpBody>,
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
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let (data, maybe_metadata) = call
        .get_flag::<Value>(engine_state, stack, "data")?
        .map(|v| (Some(HttpBody::Value(v)), None))
        .unwrap_or_else(|| match input {
            PipelineData::Value(v, metadata) => (Some(HttpBody::Value(v)), metadata),
            PipelineData::ByteStream(byte_stream, metadata) => {
                (Some(HttpBody::ByteStream(byte_stream)), metadata)
            }
            _ => (None, None),
        });
    let content_type = call
        .get_flag(engine_state, stack, "content-type")?
        .or_else(|| maybe_metadata.and_then(|m| m.content_type));

    let args = Arguments {
        url: call.req(engine_state, stack, 0)?,
        headers: call.get_flag(engine_state, stack, "headers")?,
        data,
        content_type,
        raw: call.has_flag(engine_state, stack, "raw")?,
        insecure: call.has_flag(engine_state, stack, "insecure")?,
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        timeout: call.get_flag(engine_state, stack, "max-time")?,
        full: call.has_flag(engine_state, stack, "full")?,
        allow_errors: call.has_flag(engine_state, stack, "allow-errors")?,
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
    let (requested_url, _) = http_parse_url(call, span, args.url)?;
    let redirect_mode = http_parse_redirect_mode(args.redirect)?;

    let client = http_client(args.insecure, redirect_mode, engine_state, stack)?;
    let mut request = client.delete(&requested_url);

    request = request_set_timeout(args.timeout, request)?;
    request = request_add_authorization_header(args.user, args.password, request);
    request = request_add_custom_headers(args.headers, request)?;
    let (response, request_headers) = match args.data {
        None => send_request_no_body(request, call.head, engine_state.signals()),

        Some(body) => send_request(
            engine_state,
            // Nushell allows sending body via delete method, but not via get.
            // We should probably unify the behaviour here.
            //
            // Sending body with DELETE goes against the spec, but might be useful in some cases,
            // see [force_send_body] documentation.
            request.force_send_body(),
            body,
            args.content_type,
            span,
            engine_state.signals(),
        ),
    };

    let request_flags = RequestFlags {
        raw: args.raw,
        full: args.full,
        allow_errors: args.allow_errors,
    };
    let response = response?;

    check_response_redirection(redirect_mode, span, &response)?;
    request_handle_response(
        engine_state,
        stack,
        RequestMetadata {
            requested_url: &requested_url,
            span,
            headers: request_headers,
            redirect_mode,
            flags: request_flags,
        },
        response,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(HttpDelete {})
    }
}
