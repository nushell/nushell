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

#[derive(Deserialize)]
pub struct EnterArgs {
    location: Tagged<PathBuf>,
    encoding: Option<Tagged<String>>,
}

#[async_trait]
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
        r#"Create a new shell and begin at this path.
        
Multiple encodings are supported for reading text files by using
the '--encoding <encoding>' parameter. Here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics"#
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        enter(args).await
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

async fn enter(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let scope = raw_args.scope.clone();
    let shell_manager = raw_args.shell_manager.clone();
    let head = raw_args.call_info.args.head.clone();
    let ctrl_c = raw_args.ctrl_c.clone();
    let current_errors = raw_args.current_errors.clone();
    let host = raw_args.host.clone();
    let tag = raw_args.call_info.name_tag.clone();
    let (EnterArgs { location, encoding }, _) = raw_args.process().await?;
    let location_string = location.display().to_string();
    let location_clone = location_string.clone();

    if location_string.starts_with("help") {
        let spec = location_string.split(':').collect::<Vec<&str>>();

        if spec.len() == 2 {
            let (_, command) = (spec[0], spec[1]);

            if scope.has_command(command) {
                return Ok(OutputStream::one(ReturnSuccess::action(
                    CommandAction::EnterHelpShell(
                        UntaggedValue::string(command).into_value(Tag::unknown()),
                    ),
                )));
            }
        }
        Ok(OutputStream::one(ReturnSuccess::action(
            CommandAction::EnterHelpShell(UntaggedValue::nothing().into_value(Tag::unknown())),
        )))
    } else if location.is_dir() {
        Ok(OutputStream::one(ReturnSuccess::action(
            CommandAction::EnterShell(location_clone),
        )))
    } else {
        // If it's a file, attempt to open the file as a value and enter it
        let cwd = shell_manager.path();

        let full_path = std::path::PathBuf::from(cwd);
        let span = location.span();

        let (file_extension, tagged_contents) = crate::commands::open::fetch(
            &full_path,
            &PathBuf::from(location_clone),
            span,
            encoding,
        )
        .await?;

        match tagged_contents.value {
            UntaggedValue::Primitive(Primitive::String(_)) => {
                if let Some(extension) = file_extension {
                    let command_name = format!("from {}", extension);
                    if let Some(converter) = scope.get_command(&command_name) {
                        let new_args = RawCommandArgs {
                            host,
                            ctrl_c,
                            current_errors,
                            shell_manager,
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
                            scope: scope.clone(),
                        };
                        let tag = tagged_contents.tag.clone();
                        let mut result = converter
                            .run(new_args.with_input(vec![tagged_contents]))
                            .await?;
                        let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                            result.drain_vec().await;
                        Ok(futures::stream::iter(result_vec.into_iter().map(
                            move |res| match res {
                                Ok(ReturnSuccess::Value(Value { value, .. })) => Ok(
                                    ReturnSuccess::Action(CommandAction::EnterValueShell(Value {
                                        value,
                                        tag: tag.clone(),
                                    })),
                                ),
                                x => x,
                            },
                        ))
                        .to_output_stream())
                    } else {
                        Ok(OutputStream::one(ReturnSuccess::action(
                            CommandAction::EnterValueShell(tagged_contents),
                        )))
                    }
                } else {
                    Ok(OutputStream::one(ReturnSuccess::action(
                        CommandAction::EnterValueShell(tagged_contents),
                    )))
                }
            }
            _ => Ok(OutputStream::one(ReturnSuccess::action(
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
