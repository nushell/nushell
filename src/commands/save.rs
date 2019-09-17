use crate::commands::{UnevaluatedCallInfo, WholeStreamCommand};
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub struct Save;

macro_rules! process_string {
    ($scope:tt, $input:ident, $name_tag:ident) => {{
        let mut result_string = String::new();
        for res in $input {
            match res {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => {
                    result_string.push_str(&s);
                }
                _ => {
                    break $scope Err(ShellError::labeled_error(
                        "Save could not successfully save",
                        "unexpected data during save",
                        $name_tag,
                    ));
                }
            }
        }
        Ok(result_string.into_bytes())
    }};
}

macro_rules! process_string_return_success {
    ($scope:tt, $result_vec:ident, $name_tag:ident) => {{
        let mut result_string = String::new();
        for res in $result_vec {
            match res {
                Ok(ReturnSuccess::Value(Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                })) => {
                    result_string.push_str(&s);
                }
                _ => {
                    break $scope Err(ShellError::labeled_error(
                        "Save could not successfully save",
                        "unexpected data during text save",
                        $name_tag,
                    ));
                }
            }
        }
        Ok(result_string.into_bytes())
    }};
}

macro_rules! process_binary_return_success {
    ($scope:tt, $result_vec:ident, $name_tag:ident) => {{
        let mut result_binary: Vec<u8> = Vec::new();
        for res in $result_vec {
            match res {
                Ok(ReturnSuccess::Value(Tagged {
                    item: Value::Primitive(Primitive::Binary(b)),
                    ..
                })) => {
                    for u in b.into_iter() {
                        result_binary.push(u);
                    }
                }
                _ => {
                    break $scope Err(ShellError::labeled_error(
                        "Save could not successfully save",
                        "unexpected data during binary save",
                        $name_tag,
                    ));
                }
            }
        }
        Ok(result_binary)
    }};
}

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
            .optional("path", SyntaxShape::Path)
            .switch("raw")
    }

    fn usage(&self) -> &str {
        "Save the contents of the pipeline to a file."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args.process_raw(registry, save)?.run())
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
        host,
        commands: registry,
        ..
    }: RunnableContext,
    raw_args: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut full_path = PathBuf::from(shell_manager.path());
    let name_tag = name;

    let source_map = source_map.clone();
    let stream = async_stream! {
        let input: Vec<Tagged<Value>> = input.values.collect().await;
        if path.is_none() {
            // If there is no filename, check the metadata for the anchor filename
            if input.len() > 0 {
                let anchor = input[0].anchor();
                match source_map.get(&anchor) {
                    Some(path) => match path {
                        AnchorLocation::File(file) => {
                            full_path.push(Path::new(file));
                        }
                        _ => {
                            yield Err(ShellError::labeled_error(
                                "Save requires a filepath (1)",
                                "needs path",
                                name_tag,
                            ));
                        }
                    },
                    None => {
                        eprintln!("{:?} {:?}", anchor, source_map);
                        yield Err(ShellError::labeled_error(
                            "Save requires a filepath (2)",
                            "needs path",
                            name_tag,
                        ));
                    }
                }
            } else {
                yield Err(ShellError::labeled_error(
                    "Save requires a filepath (3)",
                    "needs path",
                    name_tag,
                ));
            }
        } else {
            if let Some(file) = path {
                full_path.push(file.item());
            }
        }

        // TODO use label_break_value once it is stable:
        // https://github.com/rust-lang/rust/issues/48594
        let content : Result<Vec<u8>, ShellError> = 'scope: loop {
            break if !save_raw {
                if let Some(extension) = full_path.extension() {
                    let command_name = format!("to-{}", extension.to_str().unwrap());
                    if let Some(converter) = registry.get_command(&command_name) {
                        let new_args = RawCommandArgs {
                            host,
                            shell_manager,
                            call_info: UnevaluatedCallInfo {
                                args: crate::parser::hir::Call {
                                    head: raw_args.call_info.args.head,
                                    positional: None,
                                    named: None
                                },
                                source: raw_args.call_info.source,
                                source_map: raw_args.call_info.source_map,
                                name_tag: raw_args.call_info.name_tag,
                            }
                        };
                        let mut result = converter.run(new_args.with_input(input), &registry, false);
                        let result_vec: Vec<Result<ReturnSuccess, ShellError>> = result.drain_vec().await;
                        if converter.is_binary() {
                            process_binary_return_success!('scope, result_vec, name_tag)
                        } else {
                            process_string_return_success!('scope, result_vec, name_tag)
                        }
                    } else {
                        process_string!('scope, input, name_tag)
                    }
                } else {
                    process_string!('scope, input, name_tag)
                }
            } else {
                Ok(string_from(&input).into_bytes())
            };
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

fn string_from(input: &Vec<Tagged<Value>>) -> String {
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

    save_data
}
