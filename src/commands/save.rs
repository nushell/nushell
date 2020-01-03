use crate::commands::{UnevaluatedCallInfo, WholeStreamCommand};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::{Path, PathBuf};

pub struct Save;

macro_rules! process_string {
    ($scope:tt, $input:ident, $name_tag:ident) => {{
        let mut result_string = String::new();
        for res in $input {
            match res {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    ..
                } => {
                    result_string.push_str(&s);
                }
                _ => {
                    break $scope Err(ShellError::labeled_error(
                        "Save requires string data",
                        "consider converting data to string (see `help commands`)",
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
                Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
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
                Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(b)),
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
            .optional("path", SyntaxShape::Path, "the path to save contents to")
            .switch(
                "raw",
                "treat values as-is rather than auto-converting based on file extension",
            )
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
        host,
        ctrl_c,
        commands: registry,
        ..
    }: RunnableContext,
    raw_args: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut full_path = PathBuf::from(shell_manager.path()?);
    let name_tag = name.clone();

    let stream = async_stream! {
        let input: Vec<Value> = input.values.collect().await;
        if path.is_none() {
            // If there is no filename, check the metadata for the anchor filename
            if input.len() > 0 {
                let anchor = input[0].tag.anchor();
                match anchor {
                    Some(path) => match path {
                        AnchorLocation::File(file) => {
                            full_path.push(Path::new(&file));
                        }
                        _ => {
                            yield Err(ShellError::labeled_error(
                                "Save requires a filepath",
                                "needs path",
                                name_tag.clone(),
                            ));
                        }
                    },
                    None => {
                        yield Err(ShellError::labeled_error(
                            "Save requires a filepath",
                            "needs path",
                            name_tag.clone(),
                        ));
                    }
                }
            } else {
                yield Err(ShellError::labeled_error(
                    "Save requires a filepath",
                    "needs path",
                    name_tag.clone(),
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
                    let command_name = format!("to-{}", extension.to_string_lossy());
                    if let Some(converter) = registry.get_command(&command_name)? {
                        let new_args = RawCommandArgs {
                            host,
                            ctrl_c,
                            shell_manager,
                            call_info: UnevaluatedCallInfo {
                                args: nu_parser::hir::Call {
                                    head: raw_args.call_info.args.head,
                                    positional: None,
                                    named: None,
                                    span: Span::unknown()
                                },
                                source: raw_args.call_info.source,
                                name_tag: raw_args.call_info.name_tag,
                            }
                        };
                        let mut result = converter.run(new_args.with_input(input), &registry);
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
                Err(e) => yield Err(ShellError::labeled_error(e.to_string(), "IO error while saving", name)),
            },
            Err(e) => yield Err(e),
        }

    };

    Ok(OutputStream::new(stream))
}

fn string_from(input: &[Value]) -> String {
    let mut save_data = String::new();

    if !input.is_empty() {
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
