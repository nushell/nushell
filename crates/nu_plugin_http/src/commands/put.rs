use super::client::{
    check_response_redirection, http_client, http_parse_redirect_mode, http_parse_url,
    request_add_authorization_header, request_add_custom_headers, request_handle_response,
    request_set_timeout, send_request, HttpBody, RequestFlags,
};
use crate::HttpPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Put;

impl PluginCommand for Put {
    type Plugin = HttpPlugin;

    fn name(&self) -> &str {
        "http put"
    }

    fn signature(&self) -> Signature {
        Signature::build("http put")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .required("URL", SyntaxShape::String, "The URL to post to.")
            .optional("data", SyntaxShape::Any, "The contents of the post body. Required unless part of a pipeline.")
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
                "return values as a string instead of a table",
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

    fn description(&self) -> &str {
        "Put a body to a URL."
    }

    fn extra_description(&self) -> &str {
        "Performs HTTP PUT operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "send", "push"]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        put(engine, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put content to example.com",
                example: "http put https://www.example.com 'body'",
                result: None,
            },
            Example {
                description: "Put content to example.com, with username and password",
                example: "http put --user myuser --password mypass https://www.example.com 'body'",
                result: None,
            },
            Example {
                description: "Put content to example.com, with custom header",
                example: "http put --headers [my-header-key my-header-value] https://www.example.com",
                result: None,
            },
            Example {
                description: "Put content to example.com, with JSON body",
                example: "http put --content-type application/json https://www.example.com { field: value }",
                result: None,
            },
            Example {
                description: "Put JSON content from a pipeline to example.com",
                example: "open --raw foo.json | http put https://www.example.com",
                result: None,
            },
        ]
    }
}

struct Arguments {
    url: Value,
    headers: Option<Value>,
    data: HttpBody,
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

fn put(
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, LabeledError> {
    let (data, maybe_metadata) = call
        .opt::<Value>(1)?
        .map(|v| (HttpBody::Value(v), None))
        .unwrap_or_else(|| match input {
            PipelineData::Value(v, metadata) => (HttpBody::Value(v), metadata),
            PipelineData::ByteStream(byte_stream, metadata) => {
                (HttpBody::ByteStream(byte_stream), metadata)
            }
            _ => (HttpBody::None, None),
        });

    if let HttpBody::None = data {
        return Err(LabeledError::new(
            "Data must be provided either through pipeline or positional argument",
        ));
    }

    let content_type = call
        .get_flag("content-type")?
        .or_else(|| maybe_metadata.and_then(|m| m.content_type));

    let args = Arguments {
        url: call.req(0)?,
        headers: call.get_flag("headers")?,
        data,
        content_type,
        raw: call.has_flag("raw")?,
        insecure: call.has_flag("insecure")?,
        user: call.get_flag("user")?,
        password: call.get_flag("password")?,
        timeout: call.get_flag("max-time")?,
        full: call.has_flag("full")?,
        allow_errors: call.has_flag("allow-errors")?,
        redirect: call.get_flag("redirect-mode")?,
    };

    let span = args.url.span();
    let (requested_url, _) = http_parse_url(span, args.url)?;
    let redirect_mode = http_parse_redirect_mode(args.redirect)?;

    let client = http_client(args.insecure, redirect_mode, engine)?;
    let mut request = client.put(&requested_url);

    request = request_set_timeout(args.timeout, request)?;
    request = request_add_authorization_header(args.user, args.password, request);
    request = request_add_custom_headers(args.headers, request)?;

    let response = send_request(
        request.clone(),
        args.data,
        args.content_type,
        call.head,
        engine.signals(),
    );

    let request_flags = RequestFlags {
        raw: args.raw,
        full: args.full,
        allow_errors: args.allow_errors,
    };

    check_response_redirection(redirect_mode, span, &response)?;
    request_handle_response(
        engine,
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
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        crate::test::examples(&Put)
    }
}
