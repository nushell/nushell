use crate::prelude::*;
use nu_engine::UnevaluatedCallInfo;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::ExternalRedirection;
use nu_protocol::{
    CommandAction, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Enter;

impl WholeStreamCommand for Enter {
    fn name(&self) -> &str {
        "enter"
    }

    fn signature(&self) -> Signature {
        Signature::build("enter")
            .required(
                "location",
                SyntaxShape::FilePath,
                "the location to create a new shell from",
            )
            .named(
                "encoding",
                SyntaxShape::String,
                "encoding to use to open file",
                Some('e'),
            )
    }

    fn usage(&self) -> &str {
        "Create a new shell and begin at this path."
    }

    fn extra_usage(&self) -> &str {
        r#"Multiple encodings are supported for reading text files by using
the '--encoding <encoding>' parameter. Here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.28/encoding_rs/#statics"#
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        enter(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Enter a path as a new shell",
                example: "enter ../projectB",
                result: None,
            },
            Example {
                description: "Enter a file as a new shell",
                example: "enter package.json",
                result: None,
            },
            Example {
                description: "Enters file with iso-8859-1 encoding",
                example: "enter file.csv --encoding iso-8859-1",
                result: None,
            },
        ]
    }
}

fn enter(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let head = args.call_info.args.head.clone();
    let context = args.context.clone();
    let scope = args.scope().clone();
    let path = args.context.shell_manager().path();

    let location: Tagged<PathBuf> = args.req(0)?;
    let encoding: Option<Tagged<String>> = args.get_flag("encoding")?;
    let location_string = location.display().to_string();

    if location.is_dir() {
        Ok(ActionStream::one(ReturnSuccess::action(
            CommandAction::EnterShell(location_string),
        )))
    } else {
        // If it's a file, attempt to open the file as a value and enter it
        let cwd = path;

        let full_path = std::path::PathBuf::from(cwd);
        let span = location.span();

        let (file_extension, tagged_contents) = crate::commands::filesystem::open::fetch(
            &full_path,
            &PathBuf::from(location_string),
            span,
            encoding,
        )?;

        match tagged_contents.value {
            UntaggedValue::Primitive(Primitive::String(_)) => {
                if let Some(extension) = file_extension {
                    let command_name = format!("from {}", extension);
                    if let Some(converter) = scope.get_command(&command_name) {
                        let tag = tagged_contents.tag.clone();
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
                                name_tag: tag.clone(),
                            },
                            input: InputStream::one(tagged_contents),
                        };
                        let mut result = converter.run(new_args)?;
                        let result_vec: Vec<Value> = result.drain_vec();
                        Ok(result_vec
                            .into_iter()
                            .map(move |res| {
                                let Value { value, .. } = res;
                                Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(
                                    Value {
                                        value,
                                        tag: tag.clone(),
                                    },
                                )))
                            })
                            .into_action_stream())
                    } else {
                        Ok(ActionStream::one(ReturnSuccess::action(
                            CommandAction::EnterValueShell(tagged_contents),
                        )))
                    }
                } else {
                    Ok(ActionStream::one(ReturnSuccess::action(
                        CommandAction::EnterValueShell(tagged_contents),
                    )))
                }
            }
            _ => Ok(ActionStream::one(ReturnSuccess::action(
                CommandAction::EnterValueShell(tagged_contents),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Enter;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Enter {})
    }
}
