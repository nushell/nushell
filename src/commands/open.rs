use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::parse::span::Span;
use crate::parser::registry::{CommandConfig, NamedType};
use crate::prelude::*;
use indexmap::IndexMap;
use mime::Mime;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub struct Open;

impl Command for Open {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        open(args)
    }
    fn name(&self) -> &str {
        "open"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("raw".to_string(), NamedType::Switch);

        CommandConfig {
            name: self.name().to_string(),
            mandatory_positional: vec![],
            optional_positional: vec![],
            rest_positional: false,
            named,
            is_filter: true,
            is_sink: false,
            can_load: vec![],
            can_save: vec![],
        }
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

                    Ok((extension, s))
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
        match std::fs::read_to_string(&cwd) {
            Ok(s) => Ok((
                cwd.extension()
                    .map(|name| name.to_string_lossy().to_string()),
                s,
            )),
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

fn open(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Open requires a path or url",
            "needs path or url",
            args.name_span,
        ));
    }

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

    let file_extension = if args.has("raw") {
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

    stream.push_back(ReturnValue::Value(parse_as_value(
        file_extension,
        contents,
        span,
    )?));

    Ok(stream.boxed())
}
