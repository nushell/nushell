use crate::commands::command::SinkCommandArgs;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::lexer::Spanned;
use std::path::{Path, PathBuf};

pub fn save(args: SinkCommandArgs) -> Result<(), ShellError> {
    if args.positional.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Save requires a filepath",
            "needs path",
            args.name_span,
        ));
    }

    let cwd = args
        .ctx
        .env
        .lock()
        .unwrap()
        .front()
        .unwrap()
        .path()
        .to_path_buf();
    let mut full_path = PathBuf::from(cwd);
    match &(args.positional[0].item) {
        Value::Primitive(Primitive::String(s)) => full_path.push(Path::new(s)),
        _ => {}
    }

    let save_raw = match args.positional.get(1) {
        Some(Spanned {
            item: Value::Primitive(Primitive::String(s)),
            ..
        }) if s == "--raw" => true,
        _ => false,
    };

    let contents = match full_path.extension() {
        Some(x) if x == "toml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to toml requires a single object (or use --raw)",
                ));
            }
            toml::to_string(&args.input[0]).unwrap()
        }
        Some(x) if x == "ini" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to ini requires a single object (or use --raw)",
                ));
            }
            serde_ini::to_string(&args.input[0]).unwrap()
        }
        Some(x) if x == "json" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to json requires a single object (or use --raw)",
                ));
            }
            serde_json::to_string(&args.input[0]).unwrap()
        }
        Some(x) if x == "yml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&args.input[0]).unwrap()
        }
        Some(x) if x == "yaml" && !save_raw => {
            if args.input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yaml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&args.input[0]).unwrap()
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
