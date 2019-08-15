use crate::context::SpanSource;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::hir::SyntaxType;
use crate::parser::registry::Signature;
use crate::prelude::*;
use mime::Mime;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;
pub struct Open;

impl PerItemCommand for Open {
    fn name(&self) -> &str {
        "open"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxType::Path)
            .switch("raw")
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        run(call_info, shell_manager)
    }
}

fn run(
    call_info: &CallInfo,
    shell_manager: &ShellManager,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let cwd = PathBuf::from(shell_manager.path());
    let full_path = PathBuf::from(cwd);

    println!("{:?}", call_info.args.nth(0));

    let path = match call_info
        .args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
    {
        file => file,
    };

    let path_str = path.as_string()?;

    let (file_extension, contents, contents_tag, span_source) =
        fetch(&full_path, &path_str, path.span())?;

    let file_extension = if call_info.args.has("raw") {
        None
    } else {
        file_extension
    };

    let mut stream = VecDeque::new();

    if let Some(uuid) = contents_tag.origin {
        // If we have loaded something, track its source
        stream.push_back(ReturnSuccess::action(CommandAction::AddSpanSource(
            uuid,
            span_source,
        )))
    }

    match contents {
        Value::Primitive(Primitive::String(string)) => {
            let value = parse_as_value(file_extension, string, contents_tag, call_info.name_span)?;

            match value {
                Tagged {
                    item: Value::List(list),
                    ..
                } => {
                    for elem in list {
                        stream.push_back(ReturnSuccess::value(elem));
                    }
                }
                x => stream.push_back(ReturnSuccess::value(x)),
            }
        }

        other => stream.push_back(ReturnSuccess::value(other.tagged(contents_tag))),
    };

    Ok(stream)
}

pub fn fetch(
    cwd: &PathBuf,
    location: &str,
    span: Span,
) -> Result<(Option<String>, Value, Tag, SpanSource), ShellError> {
    let mut cwd = cwd.clone();
    if location.starts_with("http:") || location.starts_with("https:") {
        let response = reqwest::get(location);
        match response {
            Ok(mut r) => match r.headers().get("content-type") {
                Some(content_type) => {
                    let content_type = Mime::from_str(content_type.to_str().unwrap()).unwrap();
                    match (content_type.type_(), content_type.subtype()) {
                        (mime::APPLICATION, mime::XML) => Ok((
                            Some("xml".to_string()),
                            Value::string(r.text().unwrap()),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(r.url().to_string()),
                        )),
                        (mime::APPLICATION, mime::JSON) => Ok((
                            Some("json".to_string()),
                            Value::string(r.text().unwrap()),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(r.url().to_string()),
                        )),
                        (mime::APPLICATION, mime::OCTET_STREAM) => {
                            let mut buf: Vec<u8> = vec![];
                            r.copy_to(&mut buf).map_err(|_| {
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
                                SpanSource::Url(r.url().to_string()),
                            ))
                        }
                        (mime::IMAGE, image_ty) => {
                            let mut buf: Vec<u8> = vec![];
                            r.copy_to(&mut buf).map_err(|_| {
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
                                SpanSource::Url(r.url().to_string()),
                            ))
                        }
                        (mime::TEXT, mime::HTML) => Ok((
                            Some("html".to_string()),
                            Value::string(r.text().unwrap()),
                            Tag {
                                span,
                                origin: Some(Uuid::new_v4()),
                            },
                            SpanSource::Url(r.url().to_string()),
                        )),
                        (mime::TEXT, mime::PLAIN) => {
                            let path_extension = r
                                .url()
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
                                Value::string(r.text().unwrap()),
                                Tag {
                                    span,
                                    origin: Some(Uuid::new_v4()),
                                },
                                SpanSource::Url(r.url().to_string()),
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
                            SpanSource::Url(r.url().to_string()),
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
                    SpanSource::Url(r.url().to_string()),
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
        cwd.push(Path::new(location));
        if let Ok(cwd) = dunce::canonicalize(cwd) {
            match std::fs::read(&cwd) {
                Ok(bytes) => match std::str::from_utf8(&bytes) {
                    Ok(s) => Ok((
                        cwd.extension()
                            .map(|name| name.to_string_lossy().to_string()),
                        Value::string(s),
                        Tag {
                            span,
                            origin: Some(Uuid::new_v4()),
                        },
                        SpanSource::File(cwd.to_string_lossy().to_string()),
                    )),
                    Err(_) => {
                        //Non utf8 data.
                        match (bytes.get(0), bytes.get(1)) {
                            (Some(x), Some(y)) if *x == 0xff && *y == 0xfe => {
                                // Possibly UTF-16 little endian
                                let utf16 = read_le_u16(&bytes[2..]);

                                if let Some(utf16) = utf16 {
                                    match std::string::String::from_utf16(&utf16) {
                                        Ok(s) => Ok((
                                            cwd.extension()
                                                .map(|name| name.to_string_lossy().to_string()),
                                            Value::string(s),
                                            Tag {
                                                span,
                                                origin: Some(Uuid::new_v4()),
                                            },
                                            SpanSource::File(cwd.to_string_lossy().to_string()),
                                        )),
                                        Err(_) => Ok((
                                            None,
                                            Value::Binary(bytes),
                                            Tag {
                                                span,
                                                origin: Some(Uuid::new_v4()),
                                            },
                                            SpanSource::File(cwd.to_string_lossy().to_string()),
                                        )),
                                    }
                                } else {
                                    Ok((
                                        None,
                                        Value::Binary(bytes),
                                        Tag {
                                            span,
                                            origin: Some(Uuid::new_v4()),
                                        },
                                        SpanSource::File(cwd.to_string_lossy().to_string()),
                                    ))
                                }
                            }
                            (Some(x), Some(y)) if *x == 0xfe && *y == 0xff => {
                                // Possibly UTF-16 big endian
                                let utf16 = read_be_u16(&bytes[2..]);

                                if let Some(utf16) = utf16 {
                                    match std::string::String::from_utf16(&utf16) {
                                        Ok(s) => Ok((
                                            cwd.extension()
                                                .map(|name| name.to_string_lossy().to_string()),
                                            Value::string(s),
                                            Tag {
                                                span,
                                                origin: Some(Uuid::new_v4()),
                                            },
                                            SpanSource::File(cwd.to_string_lossy().to_string()),
                                        )),
                                        Err(_) => Ok((
                                            None,
                                            Value::Binary(bytes),
                                            Tag {
                                                span,
                                                origin: Some(Uuid::new_v4()),
                                            },
                                            SpanSource::File(cwd.to_string_lossy().to_string()),
                                        )),
                                    }
                                } else {
                                    Ok((
                                        None,
                                        Value::Binary(bytes),
                                        Tag {
                                            span,
                                            origin: Some(Uuid::new_v4()),
                                        },
                                        SpanSource::File(cwd.to_string_lossy().to_string()),
                                    ))
                                }
                            }
                            _ => Ok((
                                None,
                                Value::Binary(bytes),
                                Tag {
                                    span,
                                    origin: Some(Uuid::new_v4()),
                                },
                                SpanSource::File(cwd.to_string_lossy().to_string()),
                            )),
                        }
                    }
                },
                Err(_) => {
                    return Err(ShellError::labeled_error(
                        "File could not be opened",
                        "file not found",
                        span,
                    ));
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                "File could not be opened",
                "file not found",
                span,
            ));
        }
    }
}

fn read_le_u16(input: &[u8]) -> Option<Vec<u16>> {
    if input.len() % 2 != 0 || input.len() < 2 {
        None
    } else {
        let mut result = vec![];
        let mut pos = 0;
        while pos < input.len() {
            result.push(u16::from_le_bytes([input[pos], input[pos + 1]]));
            pos += 2;
        }

        Some(result)
    }
}

fn read_be_u16(input: &[u8]) -> Option<Vec<u16>> {
    if input.len() % 2 != 0 || input.len() < 2 {
        None
    } else {
        let mut result = vec![];
        let mut pos = 0;
        while pos < input.len() {
            result.push(u16::from_be_bytes([input[pos], input[pos + 1]]));
            pos += 2;
        }

        Some(result)
    }
}

pub fn parse_as_value(
    extension: Option<String>,
    contents: String,
    contents_tag: Tag,
    name_span: Span,
) -> Result<Tagged<Value>, ShellError> {
    match extension {
        Some(x) if x == "csv" => crate::commands::from_csv::from_csv_string_to_value(
            contents,
            contents_tag,
        )
        .map_err(move |_| {
            ShellError::labeled_error("Could not open as CSV", "could not open as CSV", name_span)
        }),
        Some(x) if x == "toml" => {
            crate::commands::from_toml::from_toml_string_to_value(contents, contents_tag).map_err(
                move |_| {
                    ShellError::labeled_error(
                        "Could not open as TOML",
                        "could not open as TOML",
                        name_span,
                    )
                },
            )
        }
        Some(x) if x == "json" => {
            crate::commands::from_json::from_json_string_to_value(contents, contents_tag).map_err(
                move |_| {
                    ShellError::labeled_error(
                        "Could not open as JSON",
                        "could not open as JSON",
                        name_span,
                    )
                },
            )
        }
        Some(x) if x == "ini" => crate::commands::from_ini::from_ini_string_to_value(
            contents,
            contents_tag,
        )
        .map_err(move |_| {
            ShellError::labeled_error("Could not open as INI", "could not open as INI", name_span)
        }),
        Some(x) if x == "xml" => crate::commands::from_xml::from_xml_string_to_value(
            contents,
            contents_tag,
        )
        .map_err(move |_| {
            ShellError::labeled_error("Could not open as XML", "could not open as XML", name_span)
        }),
        Some(x) if x == "yml" => {
            crate::commands::from_yaml::from_yaml_string_to_value(contents, contents_tag).map_err(
                move |_| {
                    ShellError::labeled_error(
                        "Could not open as YAML",
                        "could not open as YAML",
                        name_span,
                    )
                },
            )
        }
        Some(x) if x == "yaml" => {
            crate::commands::from_yaml::from_yaml_string_to_value(contents, contents_tag).map_err(
                move |_| {
                    ShellError::labeled_error(
                        "Could not open as YAML",
                        "could not open as YAML",
                        name_span,
                    )
                },
            )
        }
        _ => Ok(Value::string(contents).tagged(contents_tag)),
    }
}
