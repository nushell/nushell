use crate::commands::command::SinkCommandArgs;
use crate::commands::to_csv::{to_string as to_csv_to_string, value_to_csv_value};
use crate::commands::to_json::value_to_json_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::commands::to_yaml::value_to_yaml_value;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::SpanSource;
use std::path::{Path, PathBuf};

pub fn save(args: SinkCommandArgs) -> Result<(), ShellError> {
    let cwd = args.ctx.env.lock().unwrap().path().to_path_buf();
    let mut full_path = PathBuf::from(cwd);

    let save_raw = if args.call_info.args.has("raw") {
        true
    } else {
        false
    };

    if args.call_info.args.positional.is_none() {
        // If there is no filename, check the metadata for the origin filename
        if args.input.len() > 0 {
            let span = args.input[0].span();
            match span
                .source
                .map(|x| args.call_info.source_map.get(&x))
                .flatten()
            {
                Some(path) => match path {
                    SpanSource::File(file) => {
                        full_path.push(Path::new(file));
                    }
                    _ => {
                        return Err(ShellError::maybe_labeled_error(
                            "Save requires a filepath",
                            "needs path",
                            args.call_info.name_span,
                        ));
                    }
                },
                None => {
                    return Err(ShellError::maybe_labeled_error(
                        "Save requires a filepath",
                        "needs path",
                        args.call_info.name_span,
                    ));
                }
            }
        } else {
            return Err(ShellError::maybe_labeled_error(
                "Save requires a filepath",
                "needs path",
                args.call_info.name_span,
            ));
        }
    } else {
        let arg = &args.call_info.args.positional.unwrap()[0];
        let arg_span = arg.span();
        match arg.item {
            Value::Primitive(Primitive::String(ref s)) => full_path.push(Path::new(s)),
            _ => {
                return Err(ShellError::labeled_error(
                    "Save requires a string as a filepath",
                    "needs path",
                    arg_span.clone(),
                ));
            }
        }
    }

    let contents = match full_path.extension() {
        Some(x) if x == "csv" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to csv requires a single object (or use --raw)",
                ));
            }
            to_csv_to_string(&value_to_csv_value(&args.input[0])).unwrap()
        }
        Some(x) if x == "toml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to toml requires a single object (or use --raw)",
                ));
            }
            toml::to_string(&value_to_toml_value(&args.input[0])).unwrap()
        }
        Some(x) if x == "json" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to json requires a single object (or use --raw)",
                ));
            }
            serde_json::to_string(&value_to_json_value(&args.input[0])).unwrap()
        }
        Some(x) if x == "yml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&value_to_yaml_value(&args.input[0])).unwrap()
        }
        Some(x) if x == "yaml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yaml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&value_to_yaml_value(&args.input[0])).unwrap()
        }
        _ => {
            let mut save_data = String::new();
            if args.input.len() > 0 {
                let mut first = true;
                for i in args.input.iter() {
                    if !first {
                        save_data.push_str("\n");
                    } else {
                        first = false;
                    }
                    save_data.push_str(&i.as_string().unwrap());
                }
            }
            save_data
        }
    };

    let _ = std::fs::write(full_path, contents);
    Ok(())
}
