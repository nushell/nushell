use crate::commands::StaticCommand;
use crate::context::SpanSource;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{self, Signature};
use crate::prelude::*;
use mime::Mime;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

pub struct Open;

#[derive(Deserialize)]
pub struct OpenArgs {
    path: Tagged<PathBuf>,
    raw: bool,
}

impl StaticCommand for Open {
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
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, run)?.run()
    }
}

fn run(
    OpenArgs { raw, path }: OpenArgs,
    RunnableContext {
        shell_manager,
        name,
        ..
    }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let cwd = PathBuf::from(shell_manager.path());
    let full_path = PathBuf::from(cwd);

    let path_str = path.to_str().ok_or(ShellError::type_error(
        "Path",
        "invalid path".tagged(path.tag()),
    ))?;

    let (file_extension, contents, contents_tag, span_source) =
        fetch(&full_path, path_str, path.span())?;

    let file_extension = if raw { None } else { file_extension };

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
            let value = parse_as_value(file_extension, string, contents_tag, name)?;

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

    Ok(stream.boxed().to_output_stream())
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
                Err(_) => Ok((
                    None,
                    Value::Binary(bytes),
                    Tag {
                        span,
                        origin: Some(Uuid::new_v4()),
                    },
                    SpanSource::File(cwd.to_string_lossy().to_string()),
                )),
            },
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "File could not be opened",
                    "file not found",
                    span,
                ));
            }
        }
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
