use crate::commands::to_csv::{to_string as to_csv_to_string, value_to_csv_value};
use crate::commands::to_json::value_to_json_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::commands::to_yaml::value_to_yaml_value;
use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use std::path::PathBuf;

pub struct Save;

#[derive(Deserialize)]
pub struct SaveArgs {
    path: Tagged<PathBuf>,
    raw: bool,
}

impl StaticCommand for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn signature(&self) -> Signature {
        Signature::build("save")
            .required("path", SyntaxType::Path)
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

pub fn save(
    SaveArgs {
        path,
        raw: save_raw,
    }: SaveArgs,
    context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut full_path = context.cwd();
    full_path.push(path.item());

    let stream = async_stream_block! {
        let input: Vec<Tagged<Value>> = context.input.values.collect().await;

        let contents = match full_path.extension() {
            Some(x) if x == "csv" && !save_raw => {
                if input.len() != 1 {
                    yield Err(ShellError::string(
                        "saving to csv requires a single object (or use --raw)",
                    ));
                }
                to_csv_to_string(&value_to_csv_value(&input[0])).unwrap()
            }
            Some(x) if x == "toml" && !save_raw => {
                if input.len() != 1 {
                    yield Err(ShellError::string(
                        "saving to toml requires a single object (or use --raw)",
                    ));
                }
                toml::to_string(&value_to_toml_value(&input[0])).unwrap()
            }
            Some(x) if x == "json" && !save_raw => {
                if input.len() != 1 {
                    yield Err(ShellError::string(
                        "saving to json requires a single object (or use --raw)",
                    ));
                }
                serde_json::to_string(&value_to_json_value(&input[0])).unwrap()
            }
            Some(x) if x == "yml" && !save_raw => {
                if input.len() != 1 {
                    yield Err(ShellError::string(
                        "saving to yml requires a single object (or use --raw)",
                    ));
                }
                serde_yaml::to_string(&value_to_yaml_value(&input[0])).unwrap()
            }
            Some(x) if x == "yaml" && !save_raw => {
                if input.len() != 1 {
                    yield Err(ShellError::string(
                        "saving to yaml requires a single object (or use --raw)",
                    ));
                }
                serde_yaml::to_string(&value_to_yaml_value(&input[0])).unwrap()
            }
            _ => {
                let mut save_data = String::new();
                if input.len() > 0 {
                    let mut first = true;
                    for i in input.iter() {
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
    };

    Ok(OutputStream::new(stream))
}
