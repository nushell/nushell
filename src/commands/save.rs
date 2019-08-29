use crate::commands::to_csv::{to_string as to_csv_to_string, value_to_csv_value};
use crate::commands::to_json::value_to_json_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::commands::to_yaml::value_to_yaml_value;
use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub struct Save;

#[derive(Deserialize)]
pub struct SaveArgs {
    path: Option<Tagged<PathBuf>>,
    raw: bool,
}

impl WholeStreamCommand for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn signature(&self) -> Signature {
        Signature::build("save")
            .optional("path", SyntaxType::Path)
            .switch("raw")
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, save)?.run()
    }
}

fn save(
    SaveArgs {
        path,
        raw: save_raw,
    }: SaveArgs,
    RunnableContext {
        input,
        name,
        shell_manager,
        source_map,
        ..
    }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut full_path = PathBuf::from(shell_manager.path());
    let name_span = name;

    if path.is_none() {
        let source_map = source_map.clone();
        let stream = async_stream_block! {
            let input: Vec<Tagged<Value>> = input.values.collect().await;
            // If there is no filename, check the metadata for the origin filename
            if input.len() > 0 {
                let origin = input[0].origin();
                match origin.and_then(|x| source_map.get(&x)) {
                    Some(path) => match path {
                        SpanSource::File(file) => {
                            full_path.push(Path::new(file));
                        }
                        _ => {
                            yield Err(ShellError::labeled_error(
                                "Save requires a filepath",
                                "needs path",
                                name_span,
                            ));
                        }
                    },
                    None => {
                        yield Err(ShellError::labeled_error(
                            "Save requires a filepath",
                            "needs path",
                            name_span,
                        ));
                    }
                }
            } else {
                yield Err(ShellError::labeled_error(
                    "Save requires a filepath",
                    "needs path",
                    name_span,
                ));
            }

            let content = if !save_raw {
                to_string_for(full_path.extension(), &input).await
            } else {
                string_from(&input)
            };

            match content {
                Ok(save_data) => match std::fs::write(full_path, save_data) {
                    Ok(o) => o,
                    Err(e) => yield Err(ShellError::string(e.to_string())),
                },
                Err(e) => yield Err(ShellError::string(e.to_string())),
            }

        };

        Ok(OutputStream::new(stream))
    } else {
        if let Some(file) = path {
            full_path.push(file.item());
        }

        let stream = async_stream_block! {
            let input: Vec<Tagged<Value>> = input.values.collect().await;

            let content = if !save_raw {
                to_string_for(full_path.extension(), &input).await
            } else {
                string_from(&input)
            };

            match content {
                Ok(save_data) => match std::fs::write(full_path, save_data) {
                    Ok(o) => o,
                    Err(e) => yield Err(ShellError::string(e.to_string())),
                },
                Err(e) => yield Err(ShellError::string(e.to_string())),
            }

        };

        Ok(OutputStream::new(stream))
    }
}

fn string_from(input: &Vec<Tagged<Value>>) -> Result<String, ShellError> {
    let mut save_data = String::new();

    if input.len() > 0 {
        let mut first = true;
        for i in input.iter() {
            if !first {
                save_data.push_str("\n");
            } else {
                first = false;
            }
            if let Ok(data) = &i.as_string() {
                save_data.push_str(data);
            }
        }
    }

    Ok(save_data)
}

async fn to_string_for(
    ext: Option<&std::ffi::OsStr>,
    input: &Vec<Tagged<Value>>,
) -> Result<String, ShellError> {
    let contents = match ext {
        Some(x) if x == "csv" => {
            if input.len() != 1 {
                return Err(ShellError::string(
                    "saving to csv requires a single object (or use --raw)",
                ));
            }
            to_csv_to_string(&value_to_csv_value(&input[0]))?
        }
        Some(x) if x == "toml" => {
            if input.len() != 1 {
                return Err(ShellError::string(
                    "saving to toml requires a single object (or use --raw)",
                ));
            }
            toml::to_string(&value_to_toml_value(&input[0]))?
        }
        Some(x) if x == "json" => {
            if input.len() != 1 {
                return Err(ShellError::string(
                    "saving to json requires a single object (or use --raw)",
                ));
            }
            serde_json::to_string(&value_to_json_value(&input[0]))?
        }
        Some(x) if x == "yml" => {
            if input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&value_to_yaml_value(&input[0]))?
        }
        Some(x) if x == "yaml" => {
            if input.len() != 1 {
                return Err(ShellError::string(
                    "saving to yaml requires a single object (or use --raw)",
                ));
            }
            serde_yaml::to_string(&value_to_yaml_value(&input[0]))?
        }
        _ => {
            return Err(ShellError::string(
                "tried saving a single object with an unrecognized format.",
            ))
        }
    };

    Ok(contents)
}
