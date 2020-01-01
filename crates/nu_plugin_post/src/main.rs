use base64::encode;
use futures::executor::block_on;
use mime::Mime;
use nu_errors::{CoerceInto, ShellError};
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, CommandAction, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape,
    UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::{AnchorLocation, Tag, TaggedItem};
use num_traits::cast::ToPrimitive;
use std::path::PathBuf;
use std::str::FromStr;
use surf::mime;

#[derive(Clone)]
pub enum HeaderKind {
    ContentType(String),
    ContentLength(String),
}

struct Post {
    path: Option<Value>,
    has_raw: bool,
    body: Option<Value>,
    user: Option<String>,
    password: Option<String>,
    headers: Vec<HeaderKind>,
    tag: Tag,
}

impl Post {
    fn new() -> Post {
        Post {
            path: None,
            has_raw: false,
            body: None,
            user: None,
            password: None,
            headers: vec![],
            tag: Tag::unknown(),
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

        self.body = match call_info.args.nth(1).ok_or_else(|| {
            ShellError::labeled_error("No body specified", "for command", &call_info.name_tag)
        })? {
            file => Some(file.clone()),
        };

        self.user = match call_info.args.get("user") {
            Some(user) => Some(user.as_string()?),
            None => None,
        };

        self.password = match call_info.args.get("password") {
            Some(password) => Some(password.as_string()?),
            None => None,
        };

        self.headers = get_headers(&call_info)?;

        self.tag = call_info.name_tag;

        ReturnSuccess::value(UntaggedValue::nothing().into_untagged_value())
    }
}

impl Plugin for Post {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("post")
            .desc("Post content to a url and retrieve data as a table if possible.")
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
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(call_info)?;
        Ok(vec![])
    }

    fn filter(&mut self, row: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![block_on(post_helper(
            &self.path.clone().ok_or_else(|| {
                ShellError::labeled_error("expected a 'path'", "expected a 'path'", &self.tag)
            })?,
            self.has_raw,
            &self.body.clone().ok_or_else(|| {
                ShellError::labeled_error("expected a 'body'", "expected a 'body'", &self.tag)
            })?,
            self.user.clone(),
            self.password.clone(),
            &self.headers.clone(),
            row,
        ))])
    }
}

fn main() {
    serve_plugin(&mut Post::new());
}

async fn post_helper(
    path: &Value,
    has_raw: bool,
    body: &Value,
    user: Option<String>,
    password: Option<String>,
    headers: &[HeaderKind],
    row: Value,
) -> ReturnValue {
    let path_tag = path.tag.clone();
    let path_str = path.as_string()?.to_string();

    //FIXME: this is a workaround because plugins don't yet support per-item iteration
    let path_str = if path_str == "$it" {
        let path_buf = row.as_path()?;
        path_buf.display().to_string()
    } else {
        path_str
    };

    //FIXME: this is a workaround because plugins don't yet support per-item iteration
    let body = if let Ok(x) = body.as_string() {
        if x == "$it" {
            &row
        } else {
            body
        }
    } else {
        body
    };

    let (file_extension, contents, contents_tag) =
        post(&path_str, &body, user, password, &headers, path_tag.clone()).await?;

    let file_extension = if has_raw {
        None
    } else {
        // If the extension could not be determined via mimetype, try to use the path
        // extension. Some file types do not declare their mimetypes (such as bson files).
        file_extension.or_else(|| path_str.split('.').last().map(String::from))
    };

    let tagged_contents = contents.into_value(&contents_tag);

    if let Some(extension) = file_extension {
        Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
            tagged_contents,
            extension,
        )))
    } else {
        ReturnSuccess::value(tagged_contents)
    }
}

pub async fn post(
    location: &str,
    body: &Value,
    user: Option<String>,
    password: Option<String>,
    headers: &[HeaderKind],
    tag: Tag,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
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
                match value_to_json_value(&value.clone().into_untagged_value()) {
                    Ok(json_value) => match serde_json::to_string(&json_value) {
                        Ok(result_string) => {
                            let mut s = surf::post(location).body_string(result_string);

                            if let Some(login) = login {
                                s = s.set_header("Authorization", format!("Basic {}", login));
                            }
                            s.await
                        }
                        _ => {
                            return Err(ShellError::labeled_error(
                                "Could not automatically convert table",
                                "needs manual conversion",
                                tag,
                            ));
                        }
                    },
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Could not automatically convert table",
                            "needs manual conversion",
                            tag,
                        ));
                    }
                }
            }
        };
        match response {
            Ok(mut r) => match r.headers().get("content-type") {
                Some(content_type) => {
                    let content_type = Mime::from_str(content_type).map_err(|_| {
                        ShellError::labeled_error(
                            format!("Unknown MIME type: {}", content_type),
                            "unknown MIME type",
                            &tag,
                        )
                    })?;
                    match (content_type.type_(), content_type.subtype()) {
                        (mime::APPLICATION, mime::XML) => Ok((
                            Some("xml".to_string()),
                            UntaggedValue::string(r.body_string().await.map_err(|_| {
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
                            UntaggedValue::string(r.body_string().await.map_err(|_| {
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
                                UntaggedValue::binary(buf),
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
                                UntaggedValue::binary(buf),
                                Tag {
                                    anchor: Some(AnchorLocation::Url(location.to_string())),
                                    span: tag.span,
                                },
                            ))
                        }
                        (mime::TEXT, mime::HTML) => Ok((
                            Some("html".to_string()),
                            UntaggedValue::string(r.body_string().await.map_err(|_| {
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
                                .map_err(|_| {
                                    ShellError::labeled_error(
                                        format!("could not parse URL: {}", location),
                                        "could not parse URL",
                                        &tag,
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
                            UntaggedValue::string(format!(
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
                    UntaggedValue::string("No content type found".to_owned()),
                    Tag {
                        anchor: Some(AnchorLocation::Url(location.to_string())),
                        span: tag.span,
                    },
                )),
            },
            Err(_) => Err(ShellError::labeled_error(
                "URL could not be opened",
                "url not found",
                tag,
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "Expected a url",
            "needs a url",
            tag,
        ))
    }
}

// FIXME FIXME FIXME
// Ultimately, we don't want to duplicate to-json here, but we need to because there isn't an easy way to call into it, yet
pub fn value_to_json_value(v: &Value) -> Result<serde_json::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        UntaggedValue::Primitive(Primitive::Bytes(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("What about really big numbers")),
        ),
        UntaggedValue::Primitive(Primitive::Duration(secs)) => {
            serde_json::Value::Number(serde_json::Number::from(*secs))
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Decimal(f)) => serde_json::Value::Number(
            serde_json::Number::from_f64(
                f.to_f64().expect("TODO: What about really big decimals?"),
            )
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "Can not convert big decimal to f64",
                    "cannot convert big decimal to f64",
                    &v.tag,
                )
            })?,
        ),
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            serde_json::Value::Number(serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                i.tagged(&v.tag),
                "converting to JSON number",
            )?))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Pattern(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::Line(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => serde_json::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => {
                        Ok(serde_json::Value::String(string.clone()))
                    }
                    UnspannedPathMember::Int(int) => Ok(serde_json::Value::Number(
                        serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                            int.tagged(&v.tag),
                            "converting to JSON number",
                        )?),
                    )),
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        UntaggedValue::Primitive(Primitive::Path(s)) => {
            serde_json::Value::String(s.display().to_string())
        }

        UntaggedValue::Table(l) => serde_json::Value::Array(json_list(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => {
            serde_json::Value::Null
        }
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            let mut output = vec![];

            for item in b.iter() {
                output.push(serde_json::Value::Number(
                    serde_json::Number::from_f64(*item as f64).ok_or_else(|| {
                        ShellError::labeled_error(
                            "Cannot create number from from f64",
                            "cannot created number from f64",
                            &v.tag,
                        )
                    })?,
                ));
            }
            serde_json::Value::Array(output)
        }
        UntaggedValue::Row(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), value_to_json_value(v)?);
            }
            serde_json::Value::Object(m)
        }
    })
}

fn json_list(input: &[Value]) -> Result<Vec<serde_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(value_to_json_value(value)?);
    }

    Ok(out)
}

fn get_headers(call_info: &CallInfo) -> Result<Vec<HeaderKind>, ShellError> {
    let mut headers = vec![];

    match extract_header_value(&call_info, "content-type") {
        Ok(h) => {
            if let Some(ct) = h {
                headers.push(HeaderKind::ContentType(ct))
            }
        }
        Err(e) => {
            return Err(e);
        }
    };

    match extract_header_value(&call_info, "content-length") {
        Ok(h) => {
            if let Some(cl) = h {
                headers.push(HeaderKind::ContentLength(cl))
            }
        }
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
