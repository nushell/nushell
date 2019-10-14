use crate::commands::UnevaluatedCallInfo;
use crate::context::AnchorLocation;
use crate::data::Value;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry::Signature;
use crate::prelude::*;
use base64::encode;
use mime::Mime;
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
            .required("path", SyntaxShape::Any)
            .required("body", SyntaxShape::Any)
            .named("user", SyntaxShape::Any)
            .named("password", SyntaxShape::Any)
            .named("content-type", SyntaxShape::Any)
            .named("content-length", SyntaxShape::Any)
            .switch("raw")
    }

    fn usage(&self) -> &str {
        "Post content to a url and retrieve data as a table if possible."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, registry, raw_args)
    }
}

fn run(
    call_info: &CallInfo,
    registry: &CommandRegistry,
    raw_args: &RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let call_info = call_info.clone();
    let path = match call_info.args.nth(0).ok_or_else(|| {
        ShellError::labeled_error("No url specified", "for command", call_info.name_tag)
    })? {
        file => file.clone(),
    };
    let body = match call_info.args.nth(1).ok_or_else(|| {
        ShellError::labeled_error("No body specified", "for command", call_info.name_tag)
    })? {
        file => file.clone(),
    };
    let path_str = path.as_string()?;
    let path_span = path.tag();
    let has_raw = call_info.args.has("raw");
    let user = call_info.args.get("user").map(|x| x.as_string().unwrap());
    let password = call_info
        .args
        .get("password")
        .map(|x| x.as_string().unwrap());
    let registry = registry.clone();
    let raw_args = raw_args.clone();

    let headers = get_headers(&call_info)?;

    let stream = async_stream! {
        let (file_extension, contents, contents_tag, anchor_location) =
            post(&path_str, &body, user, password, &headers, path_span, &registry, &raw_args).await.unwrap();

        let file_extension = if has_raw {
            None
        } else {
            // If the extension could not be determined via mimetype, try to use the path
            // extension. Some file types do not declare their mimetypes (such as bson files).
            file_extension.or(path_str.split('.').last().map(String::from))
        };

        if contents_tag.anchor != uuid::Uuid::nil() {
            // If we have loaded something, track its source
            yield ReturnSuccess::action(CommandAction::AddAnchorLocation(
                contents_tag.anchor,
                anchor_location,
            ));
        }

        let tagged_contents = contents.tagged(contents_tag);

        if let Some(extension) = file_extension {
            let command_name = format!("from-{}", extension);
            if let Some(converter) = registry.get_command(&command_name) {
                let new_args = RawCommandArgs {
                    host: raw_args.host,
                    shell_manager: raw_args.shell_manager,
                    call_info: UnevaluatedCallInfo {
                        args: crate::parser::hir::Call {
                            head: raw_args.call_info.args.head,
                            positional: None,
                            named: None
                        },
                        source: raw_args.call_info.source,
                        source_map: raw_args.call_info.source_map,
                        name_tag: raw_args.call_info.name_tag,
                    }
                };
                let mut result = converter.run(new_args.with_input(vec![tagged_contents]), &registry, false);
                let result_vec: Vec<Result<ReturnSuccess, ShellError>> = result.drain_vec().await;
                for res in result_vec {
                    match res {
                        Ok(ReturnSuccess::Value(Tagged { item: Value::Table(list), ..})) => {
                            for l in list {
                                yield Ok(ReturnSuccess::Value(l));
                            }
                        }
                        Ok(ReturnSuccess::Value(Tagged { item, .. })) => {
                            yield Ok(ReturnSuccess::Value(Tagged { item, tag: contents_tag }));
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
            Some(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => s.clone(),
            Some(Tagged { tag, .. }) => {
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
    body: &Tagged<Value>,
    user: Option<String>,
    password: Option<String>,
    headers: &Vec<HeaderKind>,
    tag: Tag,
    registry: &CommandRegistry,
    raw_args: &RawCommandArgs,
) -> Result<(Option<String>, Value, Tag, AnchorLocation), ShellError> {
    let registry = registry.clone();
    let raw_args = raw_args.clone();
    if location.starts_with("http:") || location.starts_with("https:") {
        let login = match (user, password) {
            (Some(user), Some(password)) => Some(encode(&format!("{}:{}", user, password))),
            (Some(user), _) => Some(encode(&format!("{}:", user))),
            _ => None,
        };
        let response = match body {
            Tagged {
                item: Value::Primitive(Primitive::String(body_str)),
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
            Tagged {
                item: Value::Primitive(Primitive::Binary(b)),
                ..
            } => {
                let mut s = surf::post(location).body_bytes(b);
                if let Some(login) = login {
                    s = s.set_header("Authorization", format!("Basic {}", login));
                }
                s.await
            }
            Tagged { item, tag } => {
                if let Some(converter) = registry.get_command("to-json") {
                    let new_args = RawCommandArgs {
                        host: raw_args.host,
                        shell_manager: raw_args.shell_manager,
                        call_info: UnevaluatedCallInfo {
                            args: crate::parser::hir::Call {
                                head: raw_args.call_info.args.head,
                                positional: None,
                                named: None,
                            },
                            source: raw_args.call_info.source,
                            source_map: raw_args.call_info.source_map,
                            name_tag: raw_args.call_info.name_tag,
                        },
                    };
                    let mut result = converter.run(
                        new_args.with_input(vec![item.clone().tagged(tag.clone())]),
                        &registry,
                        false,
                    );
                    let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                        result.drain_vec().await;
                    let mut result_string = String::new();
                    for res in result_vec {
                        match res {
                            Ok(ReturnSuccess::Value(Tagged {
                                item: Value::Primitive(Primitive::String(s)),
                                ..
                            })) => {
                                result_string.push_str(&s);
                            }
                            _ => {
                                return Err(ShellError::labeled_error(
                                    "Save could not successfully save",
                                    "unexpected data during save",
                                    *tag,
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
                        *tag,
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
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    tag,
                                )
                            })?),
                            tag,
                            AnchorLocation::Url(location.to_string()),
                        )),
                        (mime::APPLICATION, mime::JSON) => Ok((
                            Some("json".to_string()),
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    tag,
                                )
                            })?),
                            tag,
                            AnchorLocation::Url(location.to_string()),
                        )),
                        (mime::APPLICATION, mime::OCTET_STREAM) => {
                            let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load binary file",
                                    "could not load",
                                    tag,
                                )
                            })?;
                            Ok((
                                None,
                                Value::binary(buf),
                                tag,
                                AnchorLocation::Url(location.to_string()),
                            ))
                        }
                        (mime::IMAGE, image_ty) => {
                            let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load image file",
                                    "could not load",
                                    tag,
                                )
                            })?;
                            Ok((
                                Some(image_ty.to_string()),
                                Value::binary(buf),
                                tag,
                                AnchorLocation::Url(location.to_string()),
                            ))
                        }
                        (mime::TEXT, mime::HTML) => Ok((
                            Some("html".to_string()),
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    tag,
                                )
                            })?),
                            tag,
                            AnchorLocation::Url(location.to_string()),
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
                                        tag,
                                    )
                                })?),
                                tag,
                                AnchorLocation::Url(location.to_string()),
                            ))
                        }
                        (ty, sub_ty) => Ok((
                            None,
                            Value::string(format!(
                                "Not yet supported MIME type: {} {}",
                                ty, sub_ty
                            )),
                            tag,
                            AnchorLocation::Url(location.to_string()),
                        )),
                    }
                }
                None => Ok((
                    None,
                    Value::string(format!("No content type found")),
                    tag,
                    AnchorLocation::Url(location.to_string()),
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
