use crate::prelude::*;
use nu_engine::{UnevaluatedCallInfo, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{
    hir::ExternalRedirection, Primitive, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::path::{Path, PathBuf};

pub struct Save;

macro_rules! process_unknown {
    ($scope:tt, $input:ident, $name_tag:ident) => {{
        if $input.len() > 0 {
            match $input[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(_)),
                    ..
                } => process_binary!($scope, $input, $name_tag),
                _ => process_string!($scope, $input, $name_tag),
            }
        } else {
            process_string!($scope, $input, $name_tag)
        }
    }};
}

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

macro_rules! process_binary {
    ($scope:tt, $input:ident, $name_tag:ident) => {{
        let mut result_binary: Vec<u8> = Vec::new();
        for res in $input {
            match res {
                Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(b)),
                    ..
                } => {
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

macro_rules! process_string_return_success {
    ($scope:tt, $result_vec:ident, $name_tag:ident) => {{
        let mut result_string = String::new();
        for res in $result_vec {
            match res {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    ..
                } => {
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
                Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(b)),
                    ..
                } => {
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

impl WholeStreamCommand for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn signature(&self) -> Signature {
        Signature::build("save")
            .optional(
                "path",
                SyntaxShape::FilePath,
                "the path to save contents to",
            )
            .switch(
                "raw",
                "treat values as-is rather than auto-converting based on file extension",
                Some('r'),
            )
            .switch("append", "append values rather than overriding", Some('a'))
    }

    fn usage(&self) -> &str {
        "Save the contents of the pipeline to a file."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        save(args)
    }
}

fn save(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let shell_manager = args.shell_manager();
    let mut full_path = PathBuf::from(shell_manager.path());
    let name_tag = args.call_info.name_tag.clone();
    let name = args.call_info.name_tag.clone();
    let context = args.context.clone();
    let scope = args.scope().clone();

    let head = args.call_info.args.head.clone();

    let path: Option<Tagged<PathBuf>> = args.opt(0)?;
    let save_raw = args.has_flag("raw");
    let append = args.has_flag("append");

    let input: Vec<Value> = args.input.collect();
    if path.is_none() {
        let mut should_return_file_path_error = true;

        // If there is no filename, check the metadata for the anchor filename
        if !input.is_empty() {
            let anchor = input[0].tag.anchor();

            if let Some(AnchorLocation::File(file)) = anchor {
                should_return_file_path_error = false;
                full_path.push(Path::new(&file));
            }
        }

        if should_return_file_path_error {
            return Err(ShellError::labeled_error(
                "Save requires a filepath",
                "needs path",
                name_tag,
            ));
        }
    } else if let Some(file) = path {
        full_path.push(file.item());
    }

    // TODO use label_break_value once it is stable:
    // https://github.com/rust-lang/rust/issues/48594
    #[allow(clippy::never_loop)]
    let content: Result<Vec<u8>, ShellError> = 'scope: loop {
        break if !save_raw {
            if let Some(extension) = full_path.extension() {
                let command_name = format!("to {}", extension.to_string_lossy());
                if let Some(converter) = scope.get_command(&command_name) {
                    let new_args = CommandArgs {
                        context,
                        call_info: UnevaluatedCallInfo {
                            args: nu_protocol::hir::Call {
                                head,
                                positional: None,
                                named: None,
                                span: Span::unknown(),
                                external_redirection: ExternalRedirection::Stdout,
                            },
                            name_tag: name_tag.clone(),
                        },
                        input: InputStream::from_stream(input.into_iter()),
                    };
                    let mut result = converter.run(new_args)?;
                    let result_vec: Vec<Value> = result.drain_vec();
                    if converter.is_binary() {
                        process_binary_return_success!('scope, result_vec, name_tag)
                    } else {
                        process_string_return_success!('scope, result_vec, name_tag)
                    }
                } else {
                    process_unknown!('scope, input, name_tag)
                }
            } else {
                process_unknown!('scope, input, name_tag)
            }
        } else {
            Ok(string_from(&input).into_bytes())
        };
    };

    shell_manager.save(&full_path, &content?, name.span, append)
}

fn string_from(input: &[Value]) -> String {
    let mut save_data = String::new();

    if !input.is_empty() {
        let mut first = true;
        for i in input {
            if !first {
                save_data.push('\n');
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

#[cfg(test)]
mod tests {
    use super::Save;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Save {})
    }
}
