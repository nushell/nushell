use crate::commands::UnevaluatedCallInfo;
use crate::data::value;
use crate::prelude::*;
use base64::encode;
use mime::Mime;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_errors::ShellError;
use nu_source::AnchorLocation;
use std::path::PathBuf;
use std::str::FromStr;
use surf::mime;

pub enum HeaderKind {
    ContentType(String),
    ContentLength(String),
}

pub struct Post;

impl PerItemCommand for Post {
    fn name(&self) -> &str {
        "post"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxShape::Any, "the URL to post to")
            .required("body", SyntaxShape::Any, "the contents of the post body")
            .named("user", SyntaxShape::Any, "the username when authenticating")
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
            )
            .named(
                "content-type",
                SyntaxShape::Any,
                "the MIME type of content to post",
            )
            .named(
                "content-length",
                SyntaxShape::Any,
                "the length of the content being posted",
            )
            .switch("raw", "return values as a string instead of a table")
    }

    fn usage(&self) -> &str {
        "Post content to a url and retrieve data as a table if possible."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, registry, raw_args)
    }
}

fn run(
    call_info: &CallInfo,
    registry: &CommandRegistry,
    raw_args: &RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let name_tag = call_info.name_tag.clone();
    let call_info = call_info.clone();
    let path =
        match call_info.args.nth(0).ok_or_else(|| {
            ShellError::labeled_error("No url specified", "for command", &name_tag)
        })? {
            file => file.clone(),
        };
    let path_tag = path.tag.clone();
    let body =
        match call_info.args.nth(1).ok_or_else(|| {
            ShellError::labeled_error("No body specified", "for command", &name_tag)
        })? {
            file => file.clone(),
        };
    let path_str = path.as_string()?.to_string();
    let has_raw = call_info.args.has("raw");
    let user = call_info
        .args
        .get("user")
        .map(|x| x.as_string().unwrap().to_string());
    let password = call_info
        .args
        .get("password")
        .map(|x| x.as_string().unwrap().to_string());
    let registry = registry.clone();
    let raw_args = raw_args.clone();

    let headers = get_headers(&call_info)?;

    let stream = async_stream! {
        let (file_extension, contents, contents_tag) =
            post(&path_str, &body, user, password, &headers, path_tag.clone(), &registry, &raw_args).await.unwrap();

        let file_extension = if has_raw {
            None
        } else {
            // If the extension could not be determined via mimetype, try to use the path
            // extension. Some file types do not declare their mimetypes (such as bson files).
            file_extension.or(path_str.split('.').last().map(String::from))
        };

        let tagged_contents = contents.into_value(&contents_tag);

        if let Some(extension) = file_extension {
            let command_name = format!("from-{}", extension);
            if let Some(converter) = registry.get_command(&command_name) {
                let new_args = RawCommandArgs {
                    host: raw_args.host,
                    ctrl_c: raw_args.ctrl_c,
                    shell_manager: raw_args.shell_manager,
                    call_info: UnevaluatedCallInfo {
                        args: nu_parser::hir::Call {
                            head: raw_args.call_info.args.head,
                            positional: None,
                            named: None,
                            span: Span::unknown()
                        },
                        source: raw_args.call_info.source,
                        name_tag: raw_args.call_info.name_tag,
                    }
                };
                let mut result = converter.run(new_args.with_input(vec![tagged_contents]), &registry);
                let result_vec: Vec<Result<ReturnSuccess, ShellError>> = result.drain_vec().await;
                for res in result_vec {
                    match res {
                        Ok(ReturnSuccess::Value(Value { value: UntaggedValue::Table(list), ..})) => {
                            for l in list {
                                yield Ok(ReturnSuccess::Value(l));
                            }
                        }
                        Ok(ReturnSuccess::Value(Value { value, .. })) => {
                            yield Ok(ReturnSuccess::Value(Value { value, tag: contents_tag.clone() }));
                        }
                        x => yield x,
                    }
                }
            } else {
                yield ReturnSuccess::value(tagged_contents);
            }
        } else {
            yield ReturnSuccess::value(tagged_contents);
        }
    };

    Ok(stream.to_output_stream())
}

fn get_headers(call_info: &CallInfo) -> Result<Vec<HeaderKind>, ShellError> {
    let mut headers = vec![];

    match extract_header_value(&call_info, "content-type") {
        Ok(h) => match h {
            Some(ct) => headers.push(HeaderKind::ContentType(ct)),
            None => {}
        },
        Err(e) => {
            return Err(e);
        }
    };

    match extract_header_value(&call_info, "content-length") {
        Ok(h) => match h {
            Some(cl) => headers.push(HeaderKind::ContentLength(cl)),
            None => {}
        },
        Err(e) => {
            return Err(e);
        }
    };

    Ok(headers)
}

fn extract_header_value(call_info: &CallInfo, key: &str) -> Result<Option<String>, ShellError> {
    if call_info.args.has(key) {
        let tagged = call_info.args.get(key);
        let val = match tagged {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            }) => s.clone(),
            Some(Value { tag, .. }) => {
                return Err(ShellError::labeled_error(
                    format!("{} not in expected format.  Expected string.", key),
                    "post error",
                    tag,
                ));
            }
            _ => {
                return Err(ShellError::labeled_error(
                    format!("{} not in expected format.  Expected string.", key),
                    "post error",
                    Tag::unknown(),
                ));
            }
        };
        return Ok(Some(val));
    }

    Ok(None)
}

pub async fn post(
    location: &str,
    body: &Value,
    user: Option<String>,
    password: Option<String>,
    headers: &Vec<HeaderKind>,
    tag: Tag,
    registry: &CommandRegistry,
    raw_args: &RawCommandArgs,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    let registry = registry.clone();
    let raw_args = raw_args.clone();
    if location.starts_with("http:") || location.starts_with("https:") {
        let login = match (user, password) {
            (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
            (Some(user), _) => Some(encode(&format!("{}:", user))),
            _ => None,
        };
        let response = match body {
            Value {
                value: UntaggedValue::Primitive(Primitive::String(body_str)),
                ..
            } => {
                let mut s = surf::post(location).body_string(body_str.to_string());
                if let Some(login) = login {
                    s = s.set_header("Authorization", format!("Basic {}", login));
                }

                for h in headers {
                    s = match h {
                        HeaderKind::ContentType(ct) => s.set_header("Content-Type", ct),
                        HeaderKind::ContentLength(cl) => s.set_header("Content-Length", cl),
                    };
                }
                s.await
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::Binary(b)),
                ..
            } => {
                let mut s = surf::post(location).body_bytes(b);
                if let Some(login) = login {
                    s = s.set_header("Authorization", format!("Basic {}", login));
                }
                s.await
            }
            Value { value, tag } => {
                if let Some(converter) = registry.get_command("to-json") {
                    let new_args = RawCommandArgs {
                        host: raw_args.host,
                        ctrl_c: raw_args.ctrl_c,
                        shell_manager: raw_args.shell_manager,
                        call_info: UnevaluatedCallInfo {
                            args: nu_parser::hir::Call {
                                head: raw_args.call_info.args.head,
                                positional: None,
                                named: None,
                                span: Span::unknown(),
                            },
                            source: raw_args.call_info.source,
                            name_tag: raw_args.call_info.name_tag,
                        },
                    };
                    let mut result = converter.run(
                        new_args.with_input(vec![value.clone().into_value(tag.clone())]),
                        &registry,
                    );
                    let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                        result.drain_vec().await;
                    let mut result_string = String::new();
                    for res in result_vec {
                        match res {
                            Ok(ReturnSuccess::Value(Value {
                                value: UntaggedValue::Primitive(Primitive::String(s)),
                                ..
                            })) => {
                                result_string.push_str(&s);
                            }
                            _ => {
                                return Err(ShellError::labeled_error(
                                    "Save could not successfully save",
                                    "unexpected data during save",
                                    tag,
                                ));
                            }
                        }
                    }

                    let mut s = surf::post(location).body_string(result_string);

                    if let Some(login) = login {
                        s = s.set_header("Authorization", format!("Basic {}", login));
                    }
                    s.await
                } else {
                    return Err(ShellError::labeled_error(
                        "Could not automatically convert table",
                        "needs manual conversion",
                        tag,
                    ));
                }
            }
        };
        match response {
            Ok(mut r) => match r.headers().get("content-type") {
                Some(content_type) => {
                    let content_type = Mime::from_str(content_type).unwrap();
                    match (content_type.type_(), content_type.subtype()) {
                        (mime::APPLICATION, mime::XML) => Ok((
                            Some("xml".to_string()),
                            value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    &tag,
                                )
                            })?),
                            Tag {
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                span: tag.span,
                            },
                        )),
                        (mime::APPLICATION, mime::JSON) => Ok((
                            Some("json".to_string()),
                            value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    &tag,
                                )
                            })?),
                            Tag {
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                span: tag.span,
                            },
                        )),
                        (mime::APPLICATION, mime::OCTET_STREAM) => {
                            let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load binary file",
                                    "could not load",
                                    &tag,
                                )
                            })?;
                            Ok((
                                None,
                                value::binary(buf),
                                Tag {
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    span: tag.span,
                                },
                            ))
                        }
                        (mime::IMAGE, image_ty) => {
                            let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load image file",
                                    "could not load",
                                    &tag,
                                )
                            })?;
                            Ok((
                                Some(image_ty.to_string()),
                                value::binary(buf),
                                Tag {
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    span: tag.span,
                                },
                            ))
                        }
                        (mime::TEXT, mime::HTML) => Ok((
                            Some("html".to_string()),
                            value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    &tag,
                                )
                            })?),
                            Tag {
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                span: tag.span,
                            },
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
                                value::string(r.body_string().await.map_err(|_| {
                                    ShellError::labeled_error(
                                        "Could not load text from remote url",
                                        "could not load",
                                        &tag,
                                    )
                                })?),
                                Tag {
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    span: tag.span,
                                },
                            ))
                        }
                        (ty, sub_ty) => Ok((
                            None,
                            value::string(format!(
                                "Not yet supported MIME type: {} {}",
                                ty, sub_ty
                            )),
                            Tag {
                                anchor: Some(AnchorLocation::Url(location.to_string())),
                                span: tag.span,
                            },
                        )),
                    }
                }
                None => Ok((
                    None,
                    value::string(format!("No content type found")),
                    Tag {
                        anchor: Some(AnchorLocation::Url(location.to_string())),
                        span: tag.span,
                    },
                )),
            },
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "URL could not be opened",
                    "url not found",
                    tag,
                ));
            }
        }
    } else {
        Err(ShellError::labeled_error(
            "Expected a url",
            "needs a url",
            tag,
        ))
    }
}
