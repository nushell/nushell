use base64::encode;
use mime::Mime;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, CommandAction, ReturnSuccess, ReturnValue, UntaggedValue, Value};
use nu_source::{AnchorLocation, Span, Tag};
use std::path::PathBuf;
use std::str::FromStr;
use surf::mime;

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

    pub fn setup(&mut self, call_info: CallInfo) -> ReturnValue {
        self.path = Some({
            let file = call_info.args.nth(0).ok_or_else(|| {
                ShellError::labeled_error(
                    "No file or directory specified",
                    "for command",
                    &call_info.name_tag,
                )
            })?;
            file.clone()
        });
        self.tag = call_info.name_tag.clone();

        self.has_raw = call_info.args.has("raw");

        self.user = match call_info.args.get("user") {
            Some(user) => Some(user.as_string()?),
            None => None,
        };

        self.password = match call_info.args.get("password") {
            Some(password) => Some(password.as_string()?),
            None => None,
        };

        ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value())
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
) -> Result<(Option<String>, Value), ShellError> {
    if let Err(e) = url::Url::parse(location) {
        return Err(ShellError::labeled_error(
            format!("Incomplete or incorrect url:\n{:?}", e),
            "expected a full url",
            span,
        ));
    }

    let login = match (user, password) {
        (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
        (Some(user), _) => Some(encode(&format!("{}:", user))),
        _ => None,
    };
    let mut response = surf::get(location);
    if let Some(login) = login {
        response = response.set_header("Authorization", format!("Basic {}", login));
    }
    let generate_error = |t: &str, e: Box<dyn std::error::Error>, span: &Span| {
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
    match response.await {
        Ok(mut r) => match r.headers().get("content-type") {
            Some(content_type) => {
                let content_type = Mime::from_str(content_type).map_err(|_| {
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
                            r.body_string()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::APPLICATION, mime::JSON) => Ok((
                        Some("json".to_string()),
                        UntaggedValue::string(
                            r.body_string()
                                .await
                                .map_err(|e| generate_error("text", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::APPLICATION, mime::OCTET_STREAM) => {
                        let buf: Vec<u8> = r
                            .body_bytes()
                            .await
                            .map_err(|e| generate_error("binary", Box::new(e), &span))?;
                        Ok((None, UntaggedValue::binary(buf).into_value(tag)))
                    }
                    (mime::IMAGE, mime::SVG) => Ok((
                        Some("svg".to_string()),
                        UntaggedValue::string(
                            r.body_string()
                                .await
                                .map_err(|e| generate_error("svg", e, &span))?,
                        )
                        .into_value(tag),
                    )),
                    (mime::IMAGE, image_ty) => {
                        let buf: Vec<u8> = r
                            .body_bytes()
                            .await
                            .map_err(|e| generate_error("image", Box::new(e), &span))?;
                        Ok((
                            Some(image_ty.to_string()),
                            UntaggedValue::binary(buf).into_value(tag),
                        ))
                    }
                    (mime::TEXT, mime::HTML) => Ok((
                        Some("html".to_string()),
                        UntaggedValue::string(
                            r.body_string()
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
                                r.body_string()
                                    .await
                                    .map_err(|e| generate_error("text", e, &span))?,
                            )
                            .into_value(tag),
                        ))
                    }
                    (_ty, _sub_ty) if has_raw => {
                        let raw_bytes = r.body_bytes().await?;

                        // For unsupported MIME types, we do not know if the data is UTF-8,
                        // so we get the raw body bytes and try to convert to UTF-8 if possible.
                        match std::str::from_utf8(&raw_bytes) {
                            Ok(response_str) => {
                                Ok((None, UntaggedValue::string(response_str).into_value(tag)))
                            }
                            Err(_) => Ok((None, UntaggedValue::binary(raw_bytes).into_value(tag))),
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
        Err(_) => Err(ShellError::labeled_error(
            "URL could not be opened",
            "url not found",
            span,
        )),
    }
}
