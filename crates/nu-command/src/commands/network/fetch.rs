use crate::prelude::*;
use base64::encode;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, ReturnValue, Value};
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::{AnchorLocation, Span, Tag};
use std::path::PathBuf;
use std::str::FromStr;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "fetch"
    }

    fn signature(&self) -> Signature {
        Signature::build("fetch")
            .desc("Load from a URL into a cell, convert to table if possible (avoid by appending '--raw').")
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
            .switch("raw", "fetch contents as text rather than a table", Some('r'))
            .filter()
    }

    fn usage(&self) -> &str {
        "Fetch the contents from a URL (HTTP GET operation)."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run_fetch(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Fetch content from url.com",
                example: "fetch url.com",
                result: None,
            },
            Example {
                description: "Fetch content from url.com, with username and password",
                example: "fetch -u myuser -p mypass url.com",
                result: None,
            },
        ]
    }
}

fn run_fetch(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let mut fetch_helper = Fetch::new();

    fetch_helper.setup(args)?;

    let runtime = tokio::runtime::Runtime::new()?;
    Ok(vec![runtime.block_on(fetch(
        &fetch_helper.path.clone().ok_or_else(|| {
            ShellError::labeled_error(
                "internal error: path not set",
                "path not set",
                &fetch_helper.tag,
            )
        })?,
        fetch_helper.has_raw,
        fetch_helper.user.clone(),
        fetch_helper.password,
    ))]
    .into_iter()
    .into_action_stream())

    //fetch.setup(callinfo)?;
}

#[derive(Default)]
pub struct Fetch {
    pub path: Option<Value>,
    pub tag: Tag,
    pub has_raw: bool,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Fetch {
    pub fn new() -> Fetch {
        Fetch {
            path: None,
            tag: Tag::unknown(),
            has_raw: false,
            user: None,
            password: None,
        }
    }

    pub fn setup(&mut self, args: CommandArgs) -> Result<(), ShellError> {
        self.path = Some({
            args.req(0).map_err(|_| {
                ShellError::labeled_error(
                    "No file or directory specified",
                    "for command",
                    &args.name_tag(),
                )
            })?
        });
        self.tag = args.name_tag();

        self.has_raw = args.has_flag("raw");

        self.user = args.get_flag("user")?;

        self.password = args.get_flag("password")?;

        Ok(())
    }
}

pub async fn fetch(
    path: &Value,
    has_raw: bool,
    user: Option<String>,
    password: Option<String>,
) -> ReturnValue {
    let path_str = path.as_string()?;
    let path_span = path.tag.span;

    let result = helper(&path_str, path_span, has_raw, user, password).await;

    if let Err(e) = result {
        return Err(e);
    }
    let (file_extension, value) = result?;

    let file_extension = if has_raw {
        None
    } else {
        // If the extension could not be determined via mimetype, try to use the path
        // extension. Some file types do not declare their mimetypes (such as bson files).
        file_extension.or_else(|| path_str.split('.').last().map(String::from))
    };

    if let Some(extension) = file_extension {
        Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
            value, extension,
        )))
    } else {
        ReturnSuccess::value(value)
    }
}

// Helper function that actually goes to retrieve the resource from the url given
// The Option<String> return a possible file extension which can be used in AutoConvert commands
async fn helper(
    location: &str,
    span: Span,
    has_raw: bool,
    user: Option<String>,
    password: Option<String>,
) -> std::result::Result<(Option<String>, Value), ShellError> {
    let url = match url::Url::parse(location) {
        Ok(u) => u,
        Err(e) => {
            return Err(ShellError::labeled_error(
                format!("Incomplete or incorrect url:\n{:?}", e),
                "expected a full url",
                span,
            ));
        }
    };

    let login = match (user, password) {
        (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
        (Some(user), _) => Some(encode(&format!("{}:", user))),
        _ => None,
    };

    let client = http_client();
    let mut request = client.get(url);

    if let Some(login) = login {
        request = request.header("Authorization", format!("Basic {}", login));
    }

    let generate_error = |t: &str, e: reqwest::Error, span: &Span| {
        ShellError::labeled_error(
            format!("Could not load {} from remote url: {:?}", t, e),
            "could not load",
            span,
        )
    };
    let tag = Tag {
        span,
        anchor: Some(AnchorLocation::Url(location.to_string())),
    };

    match request.send().await {
        Ok(r) => match r.headers().get("content-type") {
            Some(content_type) => {
                let content_type = content_type.to_str().map_err(|e| {
                    ShellError::labeled_error(e.to_string(), "MIME type were invalid", &tag)
                })?;
                let content_type = mime::Mime::from_str(content_type).map_err(|_| {
                    ShellError::labeled_error(
                        format!("MIME type unknown: {}", content_type),
                        "given unknown MIME type",
                        span,
                    )
                })?;
                match (content_type.type_(), content_type.subtype()) {
                    (mime::APPLICATION, mime::XML) => Ok((
                        Some("xml".to_string()),
                        UntaggedValue::string(
                            r.text()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::APPLICATION, mime::JSON) => Ok((
                        Some("json".to_string()),
                        UntaggedValue::string(
                            r.text()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::APPLICATION, mime::OCTET_STREAM) => {
                        let buf: Vec<u8> = r
                            .bytes()
                            .await
                            .map_err(|e| generate_error("binary", e, &span))?
                            .to_vec();
                        Ok((None, UntaggedValue::binary(buf).into_value(tag)))
                    }
                    (mime::IMAGE, mime::SVG) => Ok((
                        Some("svg".to_string()),
                        UntaggedValue::string(
                            r.text()
                                .await
                                .map_err(|e| generate_error("svg", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::IMAGE, image_ty) => {
                        let buf: Vec<u8> = r
                            .bytes()
                            .await
                            .map_err(|e| generate_error("image", e, &span))?
                            .to_vec();
                        Ok((
                            Some(image_ty.to_string()),
                            UntaggedValue::binary(buf).into_value(tag),
                        ))
                    }
                    (mime::TEXT, mime::HTML) => Ok((
                        Some("html".to_string()),
                        UntaggedValue::string(
                            r.text()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::TEXT, mime::CSV) => Ok((
                        Some("csv".to_string()),
                        UntaggedValue::string(
                            r.text()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::TEXT, mime::PLAIN) => {
                        let path_extension = url::Url::parse(location)
                            .map_err(|_| {
                                ShellError::labeled_error(
                                    format!("Cannot parse URL: {}", location),
                                    "cannot parse",
                                    span,
                                )
                            })?
                            .path_segments()
                            .and_then(|segments| segments.last())
                            .and_then(|name| if name.is_empty() { None } else { Some(name) })
                            .and_then(|name| {
                                PathBuf::from(name)
                                    .extension()
                                    .map(|name| name.to_string_lossy().to_string())
                            });

                        Ok((
                            path_extension,
                            UntaggedValue::string(
                                r.text()
                                    .await
                                    .map_err(|e| generate_error("text", e, &span))?,
                            )
                            .into_value(tag),
                        ))
                    }
                    (_ty, _sub_ty) if has_raw => {
                        let raw_bytes = r.bytes().await;
                        let raw_bytes = match raw_bytes {
                            Ok(r) => r,
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    "error with raw_bytes",
                                    e.to_string(),
                                    &span,
                                ));
                            }
                        };

                        // For unsupported MIME types, we do not know if the data is UTF-8,
                        // so we get the raw body bytes and try to convert to UTF-8 if possible.
                        match std::str::from_utf8(&raw_bytes) {
                            Ok(response_str) => {
                                Ok((None, UntaggedValue::string(response_str).into_value(tag)))
                            }
                            Err(_) => Ok((
                                None,
                                UntaggedValue::binary(raw_bytes.to_vec()).into_value(tag),
                            )),
                        }
                    }
                    (ty, sub_ty) => Err(ShellError::unimplemented(format!(
                        "Not yet supported MIME type: {} {}",
                        ty, sub_ty
                    ))),
                }
            }
            // TODO: Should this return "nothing" or Err?
            None => Ok((
                None,
                UntaggedValue::string("No content type found".to_owned()).into_value(tag),
            )),
        },
        Err(e) => Err(ShellError::labeled_error(
            "url could not be opened",
            e.to_string(),
            span,
        )),
    }
}

// Only panics if the user agent is invalid but we define it statically so either
// it always or never fails
#[allow(clippy::unwrap_used)]
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("nushell")
        .build()
        .unwrap()
}
