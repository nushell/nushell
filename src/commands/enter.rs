use crate::commands::PerItemCommand;
use crate::commands::UnevaluatedCallInfo;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, CommandAction, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use std::path::PathBuf;

pub struct Enter;

impl PerItemCommand for Enter {
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

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let raw_args = raw_args.clone();
        match call_info.args.expect_nth(0)? {
            Value {
                value: UntaggedValue::Primitive(Primitive::Path(location)),
                tag,
                ..
            } => {
                let location_string = location.display().to_string();
                let location_clone = location_string.clone();
                let tag_clone = tag.clone();

                if location.starts_with("help") {
                    let spec = location_string.split(':').collect::<Vec<&str>>();

                    let (_, command) = (spec[0], spec[1]);

                    if registry.has(command)? {
                        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                            UntaggedValue::string(command).into_value(Tag::unknown()),
                        )))]
                        .into())
                    } else {
                        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                            UntaggedValue::nothing().into_value(Tag::unknown()),
                        )))]
                        .into())
                    }
                } else if PathBuf::from(location).is_dir() {
                    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterShell(
                        location_clone,
                    )))]
                    .into())
                } else {
                    let stream = async_stream! {
                        // If it's a file, attempt to open the file as a value and enter it
                        let cwd = raw_args.shell_manager.path();

                        let full_path = std::path::PathBuf::from(cwd?);

                        let (file_extension, contents, contents_tag) =
                            crate::commands::open::fetch(
                                &full_path,
                                &location_clone,
                                tag_clone.span,
                            ).await?;

                        match contents {
                            UntaggedValue::Primitive(Primitive::String(_)) => {
                                let tagged_contents = contents.into_value(&contents_tag);

                                if let Some(extension) = file_extension {
                                    let command_name = format!("from-{}", extension);
                                    if let Some(converter) =
                                        registry.get_command(&command_name)?
                                    {
                                        let new_args = RawCommandArgs {
                                            host: raw_args.host,
                                            ctrl_c: raw_args.ctrl_c,
                                            shell_manager: raw_args.shell_manager,
                                            call_info: UnevaluatedCallInfo {
                                                args: nu_parser::hir::Call {
                                                    head: raw_args.call_info.args.head,
                                                    positional: None,
                                                    named: None,
                                                    span: Span::unknown()
                                                },
                                                source: raw_args.call_info.source,
                                                name_tag: raw_args.call_info.name_tag,
                                            },
                                        };
                                        let mut result = converter.run(
                                            new_args.with_input(vec![tagged_contents]),
                                            &registry,
                                        );
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
                    };
                    Ok(stream.to_output_stream())
                }
            }
            x => Ok(
                vec![Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(
                    x.clone(),
                )))]
                .into(),
            ),
        }
    }
}
