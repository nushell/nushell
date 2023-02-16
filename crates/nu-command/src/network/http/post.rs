use crate::formats::value_to_json_value;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

use crate::network::http::client::{
    http_client, request_add_authorization_header, request_add_custom_headers,
    request_handle_response, request_set_timeout,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "http post"
    }

    fn signature(&self) -> Signature {
        Signature::build("http post")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("path", SyntaxShape::String, "the URL to post to")
            .required("body", SyntaxShape::Any, "the contents of the post body")
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
                "content-length",
                SyntaxShape::Any,
                "the length of the content being posted",
                Some('l'),
            )
            .named(
                "timeout",
                SyntaxShape::Int,
                "timeout period in seconds",
                None,
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
            .filter()
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Post a body to a URL."
    }

    fn extra_usage(&self) -> &str {
        "Performs HTTP POST operation."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["network", "send", "push"]
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
                description: "Post content to url.com",
                example: "http post url.com 'body'",
                result: None,
            },
            Example {
                description: "Post content to url.com, with username and password",
                example: "http post -u myuser -p mypass url.com 'body'",
                result: None,
            },
            Example {
                description: "Post content to url.com, with custom header",
                example: "http post -H [my-header-key my-header-value] url.com",
                result: None,
            },
            Example {
                description: "Post content to url.com with a json body",
                example: "http post -t application/json url.com { field: value }",
                result: None,
            },
        ]
    }
}

struct Arguments {
    path: Value,
    body: Value,
    timeout: Option<Value>,
    headers: Option<Value>,
    raw: bool,
    insecure: Option<bool>,
    user: Option<String>,
    password: Option<String>,
    content_type: Option<String>,
    content_length: Option<String>,
}

#[derive(PartialEq, Eq)]
enum BodyType {
    Json,
    Form,
    Unknown,
}

fn run_post(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let args = Arguments {
        path: call.req(engine_state, stack, 0)?,
        body: call.req(engine_state, stack, 1)?,
        timeout: call.get_flag(engine_state, stack, "timeout")?,
        headers: call.get_flag(engine_state, stack, "headers")?,
        raw: call.has_flag("raw"),
        user: call.get_flag(engine_state, stack, "user")?,
        password: call.get_flag(engine_state, stack, "password")?,
        insecure: call.get_flag(engine_state, stack, "insecure")?,
        content_type: call.get_flag(engine_state, stack, "content-type")?,
        content_length: call.get_flag(engine_state, stack, "content-length")?,
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
) -> std::result::Result<PipelineData, ShellError> {
    let url_value = args.path;
    let body = args.body;
    let span = url_value.span()?;
    let requested_url = url_value.as_string()?;
    let url = match url::Url::parse(&requested_url) {
        Ok(u) => u,
        Err(_e) => {
            return Err(ShellError::UnsupportedInput(
                "Incomplete or incorrect URL. Expected a full URL, e.g., https://www.example.com"
                    .to_string(),
                format!("value: '{requested_url:?}'"),
                call.head,
                span,
            ));
        }
    };
    let user = args.user.clone();
    let password = args.password;
    let timeout = args.timeout;
    let headers = args.headers;
    let location = url;
    let raw = args.raw;

    let body_type = match &args.content_type {
        Some(it) if it == "application/json" => BodyType::Json,
        Some(it) if it == "application/x-www-form-urlencoded" => BodyType::Form,
        _ => BodyType::Unknown,
    };

    let mut request = http_client(args.insecure.is_some()).post(location);

    // set the content-type header before using e.g., request.json
    // because that will avoid duplicating the header value
    if let Some(val) = args.content_type {
        request = request.header("Content-Type", val);
    }

    match body {
        Value::Binary { val, .. } => {
            request = request.body(val);
        }
        Value::String { val, .. } => {
            request = request.body(val);
        }
        Value::Record { .. } if body_type == BodyType::Json => {
            let data = value_to_json_value(&body)?;
            request = request.json(&data);
        }
        Value::Record { .. } if body_type == BodyType::Form => {
            let data = value_to_json_value(&body)?;
            request = request.form(&data);
        }
        Value::List { vals, .. } if body_type == BodyType::Form => {
            if vals.len() % 2 != 0 {
                return Err(ShellError::IOError("unsupported body input".into()));
            }
            let data = vals
                .chunks(2)
                .map(|it| Ok((it[0].as_string()?, it[1].as_string()?)))
                .collect::<Result<Vec<(String, String)>, ShellError>>()?;
            request = request.form(&data)
        }
        _ => {
            return Err(ShellError::IOError("unsupported body input".into()));
        }
    };

    if let Some(val) = args.content_length {
        request = request.header("Content-Length", val);
    }

    request = request_set_timeout(timeout, request)?;
    request = request_add_authorization_header(user, password, request);
    request = request_add_custom_headers(headers, request)?;

    let response = request.send().and_then(|r| r.error_for_status());
    request_handle_response(engine_state, stack, span, &requested_url, raw, response)
}
