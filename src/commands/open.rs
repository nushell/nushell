use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::lexer::Spanned;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn open(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Open requires a path or url",
            "needs path or url",
            args.name_span,
        ));
    }

    let cwd = args
        .env
        .lock()
        .unwrap()
        .front()
        .unwrap()
        .path()
        .to_path_buf();
    let mut full_path = PathBuf::from(cwd);

    let (file_extension, contents) = match &args.positional[0].item {
        Value::Primitive(Primitive::String(s)) => {
            if s.starts_with("http:") || s.starts_with("https:") {
                let response = reqwest::get(s);
                match response {
                    Ok(mut r) => match r.text() {
                        Ok(s) => {
                            let fname = r
                                .url()
                                .path_segments()
                                .and_then(|segments| segments.last())
                                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                                .and_then(|name| {
                                    PathBuf::from(name)
                                        .extension()
                                        .map(|name| name.to_string_lossy().to_string())
                                });
                            (fname, s)
                        }
                        Err(_) => {
                            return Err(ShellError::labeled_error(
                                "Web page contents corrupt",
                                "received garbled data",
                                args.positional[0].span,
                            ));
                        }
                    },
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "URL could not be opened",
                            "url not found",
                            args.positional[0].span,
                        ));
                    }
                }
            } else {
                full_path.push(Path::new(&s));
                match std::fs::read_to_string(&full_path) {
                    Ok(s) => (
                        full_path
                            .extension()
                            .map(|name| name.to_string_lossy().to_string()),
                        s,
                    ),
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "File cound not be opened",
                            "file not found",
                            args.positional[0].span,
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(ShellError::labeled_error(
                "Expected string value for filename",
                "expected filename",
                args.positional[0].span,
            ));
        }
    };

    let mut stream = VecDeque::new();

    let open_raw = match args.positional.get(1) {
        Some(Spanned {
            item: Value::Primitive(Primitive::String(s)),
            ..
        }) if s == "--raw" => true,
        Some(v) => {
            return Err(ShellError::labeled_error(
                "Unknown flag for open",
                "unknown flag",
                v.span,
            ))
        }
        _ => false,
    };

    match file_extension {
        Some(x) if x == "toml" && !open_raw => {
            stream.push_back(ReturnValue::Value(
                crate::commands::from_toml::from_toml_string_to_value(contents),
            ));
        }
        Some(x) if x == "json" && !open_raw => {
            stream.push_back(ReturnValue::Value(
                crate::commands::from_json::from_json_string_to_value(contents),
            ));
        }
        Some(x) if x == "xml" && !open_raw => {
            stream.push_back(ReturnValue::Value(
                crate::commands::from_xml::from_xml_string_to_value(contents),
            ));
        }
        Some(x) if x == "yml" && !open_raw => {
            stream.push_back(ReturnValue::Value(
                crate::commands::from_yaml::from_yaml_string_to_value(contents),
            ));
        }
        Some(x) if x == "yaml" && !open_raw => {
            stream.push_back(ReturnValue::Value(
                crate::commands::from_yaml::from_yaml_string_to_value(contents),
            ));
        }
        _ => {
            stream.push_back(ReturnValue::Value(Value::Primitive(Primitive::String(
                contents,
            ))));
        }
    }

    Ok(stream.boxed())
}
