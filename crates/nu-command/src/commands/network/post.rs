use crate::prelude::*;
use base64::encode;
use mime::Mime;
use nu_engine::WholeStreamCommand;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{
    CommandAction, Primitive, ReturnSuccess, ReturnValue, UnspannedPathMember, UntaggedValue, Value,
};
use nu_protocol::{Signature, SyntaxShape};
use nu_source::{AnchorLocation, Tag, TaggedItem};
use num_traits::cast::ToPrimitive;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Command;

impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "post"
    }

    fn signature(&self) -> Signature {
        Signature::build("post")
            .desc("Post content to a URL and retrieve data as a table if possible.")
            .required("path", SyntaxShape::Any, "the URL to post to")
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
            .switch(
                "raw",
                "return values as a string instead of a table",
                Some('r'),
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Post a body to a URL (HTTP POST operation)."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run_post(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Post content to url.com",
                example: "post url.com 'body'",
                result: None,
            },
            Example {
                description: "Post content to url.com, with username and password",
                example: "post -u myuser -p mypass url.com 'body'",
                result: None,
            },
        ]
    }
}

fn run_post(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let mut helper = Post::new();

    helper.setup(args)?;

    let runtime = tokio::runtime::Runtime::new()?;
    Ok(vec![runtime.block_on(post_helper(
        &helper.path.clone().ok_or_else(|| {
            ShellError::labeled_error("expected a 'path'", "expected a 'path'", &helper.tag)
        })?,
        helper.has_raw,
        &helper.body.clone().ok_or_else(|| {
            ShellError::labeled_error("expected a 'body'", "expected a 'body'", &helper.tag)
        })?,
        helper.user.clone(),
        helper.password.clone(),
        &helper.headers,
    ))]
    .into_iter()
    .into_action_stream())

    //fetch.setup(callinfo)?;
}

#[derive(Clone)]
pub enum HeaderKind {
    ContentType(String),
    ContentLength(String),
}

#[derive(Default)]
pub struct Post {
    pub path: Option<Value>,
    pub has_raw: bool,
    pub body: Option<Value>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub headers: Vec<HeaderKind>,
    pub tag: Tag,
}

impl Post {
    pub fn new() -> Post {
        Post {
            path: None,
            has_raw: false,
            body: None,
            user: None,
            password: None,
            headers: vec![],
            tag: Tag::default(),
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

        self.body = {
            let file = args.req(1).map_err(|_| {
                ShellError::labeled_error("No body specified", "for command", &args.name_tag())
            })?;
            Some(file)
        };

        self.tag = args.name_tag();

        self.has_raw = args.has_flag("raw");

        self.user = args.get_flag("user")?;

        self.password = args.get_flag("password")?;

        self.headers = get_headers(&args)?;

        Ok(())
    }
}

pub async fn post_helper(
    path: &Value,
    has_raw: bool,
    body: &Value,
    user: Option<String>,
    password: Option<String>,
    headers: &[HeaderKind],
) -> ReturnValue {
    let path_tag = path.tag.clone();
    let path_str = path.as_string()?;

    let (file_extension, contents, contents_tag) =
        post(&path_str, body, user, password, headers, path_tag.clone()).await?;

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
                let mut s = http_client().post(location).body(body_str.to_string());
                if let Some(login) = login {
                    s = s.header("Authorization", format!("Basic {}", login));
                }

                for h in headers {
                    s = match h {
                        HeaderKind::ContentType(ct) => s.header("Content-Type", ct),
                        HeaderKind::ContentLength(cl) => s.header("Content-Length", cl),
                    };
                }

                s.send().await
            }
            Value {
                value: UntaggedValue::Primitive(Primitive::Binary(b)),
                ..
            } => {
                let mut s = http_client().post(location).body(Vec::from(&b[..]));
                if let Some(login) = login {
                    s = s.header("Authorization", format!("Basic {}", login));
                }
                s.send().await
            }
            Value { value, tag } => {
                match value_to_json_value(&value.clone().into_untagged_value()) {
                    Ok(json_value) => match serde_json::to_string(&json_value) {
                        Ok(result_string) => {
                            let mut s = http_client().post(location).body(result_string);

                            if let Some(login) = login {
                                s = s.header("Authorization", format!("Basic {}", login));
                            }
                            s.send().await
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
            Ok(r) => match r.headers().get("content-type") {
                Some(content_type) => {
                    let content_type = content_type.to_str().map_err(|e| {
                        ShellError::labeled_error(e.to_string(), "MIME type were invalid", &tag)
                    })?;
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
                            UntaggedValue::string(r.text().await.map_err(|_| {
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
                            UntaggedValue::string(r.text().await.map_err(|_| {
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
                            let buf: Vec<u8> = r
                                .bytes()
                                .await
                                .map_err(|_| {
                                    ShellError::labeled_error(
                                        "Could not load binary file",
                                        "could not load",
                                        &tag,
                                    )
                                })?
                                .to_vec();
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
                            let buf: Vec<u8> = r
                                .bytes()
                                .await
                                .map_err(|_| {
                                    ShellError::labeled_error(
                                        "Could not load image file",
                                        "could not load",
                                        &tag,
                                    )
                                })?
                                .to_vec();
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
                            UntaggedValue::string(r.text().await.map_err(|_| {
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
                                UntaggedValue::string(r.text().await.map_err(|_| {
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
        UntaggedValue::Primitive(Primitive::Filesize(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("What about really big numbers")),
        ),
        UntaggedValue::Primitive(Primitive::Duration(i)) => serde_json::Value::Number(
            serde_json::Number::from_f64(
                i.to_f64().expect("TODO: What about really big decimals?"),
            )
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "Can not convert big decimal to f64",
                    "cannot convert big decimal to f64",
                    &v.tag,
                )
            })?,
        ),
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
            serde_json::Value::Number(serde_json::Number::from(*i))
        }
        UntaggedValue::Primitive(Primitive::BigInt(i)) => {
            serde_json::Value::Number(serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                i.tagged(&v.tag),
                "converting to JSON number",
            )?))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::GlobPattern(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => serde_json::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => {
                        Ok(serde_json::Value::String(string.clone()))
                    }
                    UnspannedPathMember::Int(int) => {
                        Ok(serde_json::Value::Number(serde_json::Number::from(*int)))
                    }
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        UntaggedValue::Primitive(Primitive::FilePath(s)) => {
            serde_json::Value::String(s.display().to_string())
        }

        UntaggedValue::Table(l) => serde_json::Value::Array(json_list(l)?),
        #[cfg(feature = "dataframe")]
        UntaggedValue::DataFrame(_) | UntaggedValue::FrameStruct(_) => {
            return Err(ShellError::labeled_error(
                "Cannot convert data struct",
                "Cannot convert data struct",
                &v.tag,
            ))
        }
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => {
            serde_json::Value::Null
        }
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            let mut output = vec![];

            for item in b {
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
            for (k, v) in &o.entries {
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

fn get_headers(args: &CommandArgs) -> Result<Vec<HeaderKind>, ShellError> {
    let mut headers = vec![];

    match extract_header_value(args, "content-type") {
        Ok(h) => {
            if let Some(ct) = h {
                headers.push(HeaderKind::ContentType(ct))
            }
        }
        Err(e) => {
            return Err(e);
        }
    };

    match extract_header_value(args, "content-length") {
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

fn extract_header_value(args: &CommandArgs, key: &str) -> Result<Option<String>, ShellError> {
    if args.has_flag(key) {
        let tagged = args.get_flag(key)?;
        let val = match tagged {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(s)),
                ..
            }) => s,
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

// Only panics if the user agent is invalid but we define it statically so either
// it always or never fails
#[allow(clippy::unwrap_used)]
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("nushell")
        .build()
        .unwrap()
}
