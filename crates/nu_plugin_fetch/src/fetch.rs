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

pub async fn fetch_helper(
    path: &Value,
    has_raw: bool,
    user: Option<String>,
    password: Option<String>,
) -> ReturnValue {
    let path_str = path.as_string()?;
    let path_span = path.tag.span;

    let result = fetch(&path_str, path_span, has_raw, user, password).await;

    if let Err(e) = result {
        return Err(e);
    }
    let (file_extension, contents, contents_tag) = result?;

    let file_extension = if has_raw {
        None
    } else {
        // If the extension could not be determined via mimetype, try to use the path
        // extension. Some file types do not declare their mimetypes (such as bson files).
        file_extension.or_else(|| path_str.split('.').last().map(String::from))
    };

    let tagged_contents = contents.retag(&contents_tag);

    if let Some(extension) = file_extension {
        Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
            tagged_contents,
            extension,
        )))
    } else {
        ReturnSuccess::value(tagged_contents)
    }
}

pub async fn fetch(
    location: &str,
    span: Span,
    has_raw: bool,
    user: Option<String>,
    password: Option<String>,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    if url::Url::parse(location).is_err() {
        return Err(ShellError::labeled_error(
            "Incomplete or incorrect url",
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
                        UntaggedValue::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Some(AnchorLocation::Url(location.to_string())),
                            data_color: None,
                        },
                    )),
                    (mime::APPLICATION, mime::JSON) => Ok((
                        Some("json".to_string()),
                        UntaggedValue::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Some(AnchorLocation::Url(location.to_string())),
                            data_color: None,
                        },
                    )),
                    (mime::APPLICATION, mime::OCTET_STREAM) => {
                        let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load binary file",
                                "could not load",
                                span,
                            )
                        })?;
                        Ok((
                            None,
                            UntaggedValue::binary(buf),
                            Tag {
                                span,
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                data_color: None,
                            },
                        ))
                    }
                    (mime::IMAGE, mime::SVG) => Ok((
                        Some("svg".to_string()),
                        UntaggedValue::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load svg from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Some(AnchorLocation::Url(location.to_string())),
                            data_color: None,
                        },
                    )),
                    (mime::IMAGE, image_ty) => {
                        let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load image file",
                                "could not load",
                                span,
                            )
                        })?;
                        Ok((
                            Some(image_ty.to_string()),
                            UntaggedValue::binary(buf),
                            Tag {
                                span,
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                data_color: None,
                            },
                        ))
                    }
                    (mime::TEXT, mime::HTML) => Ok((
                        Some("html".to_string()),
                        UntaggedValue::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Some(AnchorLocation::Url(location.to_string())),
                            data_color: None,
                        },
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
                            UntaggedValue::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    span,
                                )
                            })?),
                            Tag {
                                span,
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                data_color: None,
                            },
                        ))
                    }
                    (_ty, _sub_ty) if has_raw => {
                        let raw_bytes = r.body_bytes().await?;

                        // For unsupported MIME types, we do not know if the data is UTF-8,
                        // so we get the raw body bytes and try to convert to UTF-8 if possible.
                        match std::str::from_utf8(&raw_bytes) {
                            Ok(response_str) => Ok((
                                None,
                                UntaggedValue::string(response_str),
                                Tag {
                                    span,
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    data_color: None,
                                },
                            )),
                            Err(_) => Ok((
                                None,
                                UntaggedValue::binary(raw_bytes),
                                Tag {
                                    span,
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    data_color: None,
                                },
                            )),
                        }
                    }
                    (ty, sub_ty) => Ok((
                        None,
                        UntaggedValue::string(format!(
                            "Not yet supported MIME type: {} {}",
                            ty, sub_ty
                        )),
                        Tag {
                            span,
                            anchor: Some(AnchorLocation::Url(location.to_string())),
                            data_color: None,
                        },
                    )),
                }
            }
            None => Ok((
                None,
                UntaggedValue::string("No content type found".to_owned()),
                Tag {
                    span,
                    anchor: Some(AnchorLocation::Url(location.to_string())),
                    data_color: None,
                },
            )),
        },
        Err(_) => Err(ShellError::labeled_error(
            "URL could not be opened",
            "url not found",
            span,
        )),
    }
}
