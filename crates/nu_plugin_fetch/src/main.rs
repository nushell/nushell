use futures::executor::block_on;
use mime::Mime;
use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, CommandAction, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue,
    Value,
};
use nu_source::{AnchorLocation, Span, Tag};
use std::path::PathBuf;
use std::str::FromStr;
use surf::mime;

struct Fetch {
    path: Option<Value>,
    has_raw: bool,
}

impl Fetch {
    fn new() -> Fetch {
        Fetch {
            path: None,
            has_raw: false,
        }
    }

    fn setup(&mut self, call_info: CallInfo) -> ReturnValue {
        self.path = Some(
            match call_info.args.nth(0).ok_or_else(|| {
                ShellError::labeled_error(
                    "No file or directory specified",
                    "for command",
                    &call_info.name_tag,
                )
            })? {
                file => file.clone(),
            },
        );

        self.has_raw = call_info.args.has("raw");

        ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value())
    }
}

impl Plugin for Fetch {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("fetch")
            .desc("Load from a URL into a cell, convert to table if possible (avoid by appending '--raw')")
            .required(
                "path",
                SyntaxShape::Path,
                "the URL to fetch the contents from",
            )
            .switch("raw", "fetch contents as text rather than a table")
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(callinfo)?;
        Ok(vec![])
    }

    fn filter(&mut self, value: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![block_on(fetch_helper(
            &self.path.clone().ok_or_else(|| {
                ShellError::labeled_error(
                    "internal error: path not set",
                    "path not set",
                    &value.tag,
                )
            })?,
            self.has_raw,
            value,
        ))])
    }
}

fn main() {
    serve_plugin(&mut Fetch::new());
}

async fn fetch_helper(path: &Value, has_raw: bool, row: Value) -> ReturnValue {
    let path_buf = path.as_path()?;
    let path_str = path_buf.display().to_string();

    //FIXME: this is a workaround because plugins don't yet support per-item iteration
    let path_str = if path_str == "$it" {
        let path_buf = row.as_path()?;
        path_buf.display().to_string()
    } else {
        path_str
    };

    let path_span = path.tag.span;

    let result = fetch(&path_str, path_span).await;

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
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    if url::Url::parse(location).is_err() {
        return Err(ShellError::labeled_error(
            "Incomplete or incorrect url",
            "expected a full url",
            span,
        ));
    }

    let response = surf::get(location).await;
    match response {
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
                            },
                        ))
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
