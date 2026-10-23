use nu_engine::{command_prelude::*, get_full_help};
use nu_protocol::shell_error::generic::GenericError;

use super::client::{RedirectMode, add_unix_socket_flag};
use super::get::run_get;
use super::patch::run_patch_with_data;
use super::post::run_post_with_data;
use super::put::run_put_with_data;
use super::{HttpDelete, HttpGet, HttpHead, HttpOptions, HttpPatch, HttpPost, HttpPut};

const METHODS: &[&str] = &["get", "head", "options", "delete", "post", "put", "patch"];

#[derive(Clone)]
pub struct Http;

impl Command for Http {
    fn name(&self) -> &str {
        "http"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build("http")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            // common to get more than help. Get by default
            .optional(
                "URL",
                SyntaxShape::String,
                "The URL to fetch the contents from.",
            )
            // post
            .optional(
                "data",
                SyntaxShape::Any,
                "The request body. For POST, PUT, and PATCH, required unless part of a pipeline.",
            )
            .param(
                Flag::new("method")
                    .arg(SyntaxShape::String)
                .desc("Select GET, HEAD, OPTIONS, DELETE, POST, PUT, or PATCH (case-insensitive). POST, PUT, and PATCH require a body; DELETE accepts one. GET, HEAD, and OPTIONS reject bodies. Piped `null` counts as no body; a piped body without --method is ignored for legacy GET selection.")
                    .completion(Completion::new_list(METHODS)),
            )
            .named(
                "content-type",
                SyntaxShape::Any,
                "The MIME type of the request body.",
                Some('t'),
            )
            // common
            .named(
                "user",
                SyntaxShape::Any,
                "The username when authenticating.",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::Any,
                "The password when authenticating.",
                Some('p'),
            )
            .named(
                "max-time",
                SyntaxShape::Duration,
                "Max duration before timeout occurs.",
                Some('m'),
            )
            .named(
                "headers",
                SyntaxShape::Any,
                "Custom headers you want to add.",
                Some('H'),
            )
            .switch(
                "raw",
                "Fetch contents as text rather than a table.",
                Some('r'),
            )
            .switch(
                "insecure",
                "Allow insecure server connections when using SSL.",
                Some('k'),
            )
            .switch(
                "full",
                "Returns the record, containing metainformation about the exchange in addition to \
                 the response.
                Returning record fields:
                - urls: list of url redirects this command had to make to get to the destination
                - headers.request: list of headers passed when doing the request
                - headers.response: list of received headers
                - body: the http body of the response
                - status: the http status of the response.",
                Some('f'),
            )
            .switch(
                "allow-errors",
                "Do not fail if the server returns an error code.",
                Some('e'),
            )
            .switch("pool", "Using a global pool as a client.", None)
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
            .category(Category::Network);
        add_unix_socket_flag(sig)
    }

    fn description(&self) -> &str {
        "Various commands for working with http methods."
    }

    fn extra_description(&self) -> &str {
        "Use one of the HTTP subcommands or --method to select a method."
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
        let url = call.opt::<Spanned<String>>(engine_state, stack, 0)?;
        // Evaluate the optional body once: `null` preserves default GET inference.
        let data = call.opt::<Value>(engine_state, stack, 1)?;
        let method: Option<Spanned<String>> = call.get_flag(engine_state, stack, "method")?;

        // prefer stricter calls over aliasing with variables
        if let Some(Spanned { item: method, span }) = &url
            && let lower = method.to_lowercase()
            && METHODS.contains(&lower.as_str())
        {
            return Err(ShellError::Generic(
                GenericError::new(
                    "Invalid command construction",
                    format!(
                        "Using {lower:?} dynamically is bad command construction. You are providing it to the `url` positional argument of `http`"
                    ),
                    *span,
                )
                .with_help(format!("Prefer to use `http {lower}` directly, or use `http --method $method URL` for a dynamic method")),
            ));
        }

        if let Some(method) = method {
            return run_with_method(engine_state, stack, call, input, method, data);
        }

        match (url, data) {
            (Some(_), Some(data)) => {
                run_post_with_data(engine_state, stack, call, input, Some(Some(data)))
            }
            (Some(_), None) => run_get(engine_state, stack, call, input),
            (None, Some(_)) => Err(ShellError::NushellFailed {
                msg: (String::from("Default verb is get with a payload. Impossible state")),
            }),
            (None, None) => Ok(Value::string(
                get_full_help(self, engine_state, stack, call.head),
                call.head,
            )
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get content from example.com with default verb.",
                example: "http https://www.example.com",
                result: None,
            },
            Example {
                description: "Post content to example.com with default verb.",
                example: "http https://www.example.com 'body'",
                result: None,
            },
            Example {
                description: "Get content from example.com with explicit verb.",
                example: "http get https://www.example.com",
                result: None,
            },
            Example {
                description: "Select a request method from a variable.",
                example: "let method = 'put'; http --method $method https://www.example.com 'body'",
                result: None,
            },
            Example {
                description: "Send a JSON body from the pipeline with a selected method.",
                example: "{name: 'updated'} | to json | http --method patch https://www.example.com",
                result: None,
            },
        ]
    }
}

/// Reuse the dedicated commands so authentication, redirects, streaming, and
/// method-specific response handling stay consistent with the subcommands.
fn run_with_method(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    mut input: PipelineData,
    method: Spanned<String>,
    mut data: Option<Value>,
) -> Result<PipelineData, ShellError> {
    let method_name = method.item.to_ascii_lowercase();
    let (command, accepts_body): (&dyn Command, bool) = match method_name.as_str() {
        "get" => (&HttpGet, false),
        "head" => (&HttpHead, false),
        "options" => (&HttpOptions, false),
        "delete" => (&HttpDelete, true),
        "post" => (&HttpPost, true),
        "put" => (&HttpPut, true),
        "patch" => (&HttpPatch, true),
        _ => {
            return Err(ShellError::Generic(
                GenericError::new(
                    "Unsupported HTTP method",
                    format!("Unsupported HTTP method {:?}", method.item),
                    method.span,
                )
                .with_help(format!("Supported methods: {}", METHODS.join(", "))),
            ));
        }
    };

    if !call.has_positional_args(stack, 0) {
        return Err(ShellError::MissingParameter {
            param_name: "URL".into(),
            span: call.head,
        });
    }

    let has_data = data.is_some();
    // Wrapper functions commonly pipe an optional body. A null value is no
    // body, just like an empty pipeline, and must remain valid for GET/HEAD/OPTIONS.
    if !accepts_body && (has_data || !input.is_nothing()) {
        return Err(ShellError::Generic(GenericError::new(
            format!(
                "HTTP {} does not accept a request body",
                method_name.to_ascii_uppercase()
            ),
            "Remove the body or select POST, PUT, PATCH, or DELETE",
            method.span,
        )));
    }

    // `http delete` accepts --data rather than a second positional argument.
    // Adapt the generic command's positional body into the existing piped-body
    // path, with the same precedence over pipeline input as POST/PUT/PATCH.
    if method_name == "delete" {
        if let Some(data) = data.take() {
            input = data.into_pipeline_data();
        } else if matches!(input, PipelineData::ListStream(..)) {
            return Err(ShellError::Generic(GenericError::new(
                "HTTP DELETE cannot send a list stream as a request body",
                "Convert the stream to text, JSON, or bytes before piping it to HTTP DELETE",
                input.span().unwrap_or(call.head),
            )));
        }
    }

    if input.is_nothing() {
        input = PipelineData::empty();
    }

    match method_name.as_str() {
        "post" => run_post_with_data(engine_state, stack, call, input, Some(data)),
        "put" => run_put_with_data(engine_state, stack, call, input, Some(data)),
        "patch" => run_patch_with_data(engine_state, stack, call, input, Some(data)),
        _ => command.run(engine_state, stack, call, input),
    }
}
