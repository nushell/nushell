use crate::commands::UnevaluatedCallInfo;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    CommandAction, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Enter;

#[derive(Deserialize)]
pub struct EnterArgs {
    location: Tagged<PathBuf>,
}

#[async_trait]
impl WholeStreamCommand for Enter {
    fn name(&self) -> &str {
        "enter"
    }

    fn signature(&self) -> Signature {
        Signature::build("enter").required(
            "location",
            SyntaxShape::Path,
            "the location to create a new shell from",
        )
    }

    fn usage(&self) -> &str {
        "Create a new shell and begin at this path."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        enter(args, registry)
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
        ]
    }
}

fn enter(raw_args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let scope = raw_args.call_info.scope.clone();
        let shell_manager = raw_args.shell_manager.clone();
        let head = raw_args.call_info.args.head.clone();
        let ctrl_c = raw_args.ctrl_c.clone();
        let current_errors = raw_args.current_errors.clone();
        let host = raw_args.host.clone();
        let tag = raw_args.call_info.name_tag.clone();
        let (EnterArgs { location }, _) = raw_args.process(&registry).await?;
        let location_string = location.display().to_string();
        let location_clone = location_string.clone();

        if location_string.starts_with("help") {
            let spec = location_string.split(':').collect::<Vec<&str>>();

            if spec.len() == 2 {
                let (_, command) = (spec[0], spec[1]);

                if registry.has(command) {
                    yield Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                        UntaggedValue::string(command).into_value(Tag::unknown()),
                    )));
                    return;
                }
            }
            yield Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                UntaggedValue::nothing().into_value(Tag::unknown()),
            )));
        } else if location.is_dir() {
            yield Ok(ReturnSuccess::Action(CommandAction::EnterShell(
                location_clone,
            )));
        } else {
            // If it's a file, attempt to open the file as a value and enter it
            let cwd = shell_manager.path();

            let full_path = std::path::PathBuf::from(cwd);

            let (file_extension, contents, contents_tag) =
                crate::commands::open::fetch(
                    &full_path,
                    &PathBuf::from(location_clone),
                    tag.span,
                ).await?;

            match contents {
                UntaggedValue::Primitive(Primitive::String(_)) => {
                    let tagged_contents = contents.into_value(&contents_tag);

                    if let Some(extension) = file_extension {
                        let command_name = format!("from {}", extension);
                        if let Some(converter) =
                            registry.get_command(&command_name)
                        {
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
                                        is_last: false,
                                    },
                                    name_tag: tag.clone(),
                                    scope: scope.clone()
                                },
                            };
                            let mut result = converter.run(
                                new_args.with_input(vec![tagged_contents]),
                                &registry,
                            ).await;
                            let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                                result.drain_vec().await;
                            for res in result_vec {
                                match res {
                                    Ok(ReturnSuccess::Value(Value {
                                        value,
                                        ..
                                    })) => {
                                        yield Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(
                                            Value {
                                                value,
                                                tag: contents_tag.clone(),
                                            })));
                                    }
                                    x => yield x,
                                }
                            }
                        } else {
                            yield Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(tagged_contents)));
                        }
                    } else {
                        yield Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(tagged_contents)));
                    }
                }
                _ => {
                    let tagged_contents = contents.into_value(contents_tag);

                    yield Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(tagged_contents)));
                }
            }
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Enter;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Enter {})
    }
}
