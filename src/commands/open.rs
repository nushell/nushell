use crate::errors::ShellError;
use crate::object::{Primitive, Switch, Value};
use crate::parser::parse::span::Span;
use crate::parser::registry::NamedType;
use crate::prelude::*;
use mime::Mime;
use std::path::{Path, PathBuf};
use std::str::FromStr;

command! {
    Open as open(args, --raw: Switch) {
        let span = args.name_span;

        let cwd = args
            .env
            .lock()
            .unwrap()
            .front()
            .unwrap()
            .path()
            .to_path_buf();

        let full_path = PathBuf::from(cwd);

        let (file_extension, contents) = match &args.expect_nth(0)?.item {
            Value::Primitive(Primitive::String(s)) => fetch(&full_path, s, args.expect_nth(0)?.span)?,
            _ => {
                return Err(ShellError::labeled_error(
                    "Expected string value for filename",
                    "expected filename",
                    args.expect_nth(0)?.span,
                ));
            }
        };

        let mut stream = VecDeque::new();

        let file_extension = if raw.is_present() {
            None
        } else if args.has("json") {
            Some("json".to_string())
        } else if args.has("xml") {
            Some("xml".to_string())
        } else if args.has("ini") {
            Some("ini".to_string())
        } else if args.has("yaml") {
            Some("yaml".to_string())
        } else if args.has("toml") {
            Some("toml".to_string())
        } else {
            if let Some(ref named_args) = args.args.named {
                for named in named_args.iter() {
                    return Err(ShellError::labeled_error(
                        "Unknown flag for open",
                        "unknown flag",
                        named.1.span.clone(),
                    ));
                }
                file_extension
            } else {
                file_extension
            }
        };

        stream.push_back(ReturnSuccess::value(parse_as_value(
            file_extension,
            contents,
            span,
        )?));

        stream
    }
}

pub fn fetch(
    cwd: &PathBuf,
    location: &str,
    span: Span,
) -> Result<(Option<String>, String), ShellError> {
    let mut cwd = cwd.clone();
    if location.starts_with("http:") || location.starts_with("https:") {
        let response = reqwest::get(location);
        match response {
            Ok(mut r) => match r.text() {
                Ok(s) => {
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

                    let extension = match r.headers().get("content-type") {
                        Some(content_type) => {
                            let content_type =
                                Mime::from_str(content_type.to_str().unwrap()).unwrap();
                            match (content_type.type_(), content_type.subtype()) {
                                (mime::APPLICATION, mime::XML) => Some("xml".to_string()),
                                (mime::APPLICATION, mime::JSON) => Some("json".to_string()),
                                _ => path_extension,
                            }
                        }
                        None => path_extension,
                    };

                    Ok((extension, Value::string(s)))
                }
                Err(_) => {
                    return Err(ShellError::labeled_error(
                        "Web page contents corrupt",
                        "received garbled data",
                        span,
                    ));
                }
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
                )),
                Err(_) => Ok((None, Value::Binary(bytes))),
            },
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "File cound not be opened",
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
    name_span: Option<Span>,
) -> Result<Value, ShellError> {
    match extension {
        Some(x) if x == "toml" => crate::commands::from_toml::from_toml_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as TOML",
                    "could not open as TOML",
                    name_span,
                )
            }),
        Some(x) if x == "json" => crate::commands::from_json::from_json_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as JSON",
                    "could not open as JSON",
                    name_span,
                )
            }),
        Some(x) if x == "ini" => crate::commands::from_ini::from_ini_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as INI",
                    "could not open as INI",
                    name_span,
                )
            }),
        Some(x) if x == "xml" => crate::commands::from_xml::from_xml_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as XML",
                    "could not open as XML",
                    name_span,
                )
            }),
        Some(x) if x == "yml" => crate::commands::from_yaml::from_yaml_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as YAML",
                    "could not open as YAML",
                    name_span,
                )
            }),
        Some(x) if x == "yaml" => crate::commands::from_yaml::from_yaml_string_to_value(contents)
            .map_err(move |_| {
                ShellError::maybe_labeled_error(
                    "Could not open as YAML",
                    "could not open as YAML",
                    name_span,
                )
            }),
        _ => Ok(Value::string(contents)),
    }
}
