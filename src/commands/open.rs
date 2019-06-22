use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::registry::{CommandConfig, NamedType};
use crate::prelude::*;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

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
        }
    }
}

fn open(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::string("open requires a filepath or url"));
    }

    let cwd = args
        .env
        .lock()
        .unwrap()
        .first()
        .unwrap()
        .path()
        .to_path_buf();
    let mut full_path = PathBuf::from(cwd);

    let (file_extension, contents) = match &args.expect_nth(0)?.item {
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
                                args.expect_nth(0)?.span,
                            ));
                        }
                    },
                    Err(_) => {
                        return Err(ShellError::labeled_error(
                            "URL could not be opened",
                            "url not found",
                            args.expect_nth(0)?.span,
                        ));
                    }
                }
            } else {
                full_path.push(Path::new(s));
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
                            args.expect_nth(0)?.span,
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(ShellError::labeled_error(
                "Expected string value for filename",
                "expected filename",
                args.expect_nth(0)?.span,
            ));
        }
    };

    let mut stream = VecDeque::new();

    let open_raw = args.has("raw");

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
            stream.push_back(ReturnValue::Value(Value::string(contents)));
        }
    }

    Ok(stream.boxed())
}
