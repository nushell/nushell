use crate::context::SpanSource;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::hir::SyntaxType;
use crate::parser::registry::Signature;
use crate::prelude::*;
use base64::encode;
use mime::Mime;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use surf::mime;
use uuid::Uuid;
pub struct Post;

impl PerItemCommand for Post {
    fn name(&self) -> &str {
        "post"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxType::Any)
            .required("body", SyntaxType::Any)
            .named("user", SyntaxType::Any)
            .named("password", SyntaxType::Any)
            .switch("raw")
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, shell_manager)
    }
}

fn run(call_info: &CallInfo, shell_manager: &ShellManager) -> Result<OutputStream, ShellError> {
    let cwd = PathBuf::from(shell_manager.path());
    let full_path = PathBuf::from(cwd);

    let path = match call_info
        .args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
    {
        file => file,
    };
    let body = match call_info
        .args
        .nth(1)
        .ok_or_else(|| ShellError::string(&format!("No body specified")))?
    {
        file => file,
    };
    let path_str = path.as_string()?;
    let body_str = body.as_string()?;
    let path_span = path.span();
    let name_span = call_info.name_span;
    let has_raw = call_info.args.has("raw");
    let user = call_info.args.get("user").map(|x| x.as_string().unwrap());
    let password = call_info
        .args
        .get("password")
        .map(|x| x.as_string().unwrap());

    //r#"{"query": "query { viewer { name, } }"}"#.to_string()
    let stream = async_stream_block! {
        let (file_extension, contents, contents_tag, span_source) =
            post(&path_str, body_str, user, password, path_span).await.unwrap();

        //println!("{:?}", contents);

        yield ReturnSuccess::value(contents.tagged(contents_tag));
    };

    Ok(stream.to_output_stream())
}

pub async fn post(
    location: &str,
    body: String,
    user: Option<String>,
    password: Option<String>,
    span: Span,
) -> Result<(Option<String>, Value, Tag, SpanSource), ShellError> {
    if location.starts_with("http:") || location.starts_with("https:") {
        let login = encode(&format!("{}:{}", user.unwrap(), password.unwrap()));
        let response = surf::post(location)
            .body_string(body)
            .set_header("Authorization", format!("Basic {}", login))
            .await;
        match response {
            Ok(mut r) => match r.headers().get("content-type") {
                Some(content_type) => {
                    let content_type = Mime::from_str(content_type).unwrap();
                    match (content_type.type_(), content_type.subtype()) {
                        (mime::APPLICATION, mime::XML) => Ok((
                            Some("xml".to_string()),
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    span,
                                )
                            })?),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(location.to_string()),
                        )),
                        (mime::APPLICATION, mime::JSON) => Ok((
                            Some("json".to_string()),
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    span,
                                )
                            })?),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(location.to_string()),
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
                                Value::Binary(buf),
                                Tag {
                                    span,
                                    origin: Some(Uuid::new_v4()),
                                },
                                SpanSource::Url(location.to_string()),
                            ))
                        }
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
                                Value::Binary(buf),
                                Tag {
                                    span,
                                    origin: Some(Uuid::new_v4()),
                                },
                                SpanSource::Url(location.to_string()),
                            ))
                        }
                        (mime::TEXT, mime::HTML) => Ok((
                            Some("html".to_string()),
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    span,
                                )
                            })?),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(location.to_string()),
                        )),
                        (mime::TEXT, mime::PLAIN) => {
                            let path_extension = url::Url::parse(location)
                                .unwrap()
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
                                Value::string(r.body_string().await.map_err(|_| {
                                    ShellError::labeled_error(
                                        "Could not load text from remote url",
                                        "could not load",
                                        span,
                                    )
                                })?),
                                Tag {
                                    span,
                                    origin: Some(Uuid::new_v4()),
                                },
                                SpanSource::Url(location.to_string()),
                            ))
                        }
                        (ty, sub_ty) => Ok((
                            None,
                            Value::string(format!(
                                "Not yet supported MIME type: {} {}",
                                ty, sub_ty
                            )),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(location.to_string()),
                        )),
                    }
                }
                None => Ok((
                    None,
                    Value::string(format!("No content type found")),
                    Tag {
                        span,
                        origin: Some(Uuid::new_v4()),
                    },
                    SpanSource::Url(location.to_string()),
                )),
            },
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "URL could not be opened",
                    "url not found",
                    span,
                ));
            }
        }
    } else {
        Err(ShellError::labeled_error(
            "Expected a url",
            "needs a url",
            span,
        ))
    }
}
