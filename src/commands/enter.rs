use crate::commands::command::CommandAction;
use crate::commands::PerItemCommand;
use crate::commands::UnevaluatedCallInfo;
use crate::data::meta::Span;
use crate::errors::ShellError;
use crate::parser::registry;
use crate::prelude::*;
use std::path::PathBuf;

pub struct Enter;

impl PerItemCommand for Enter {
    fn name(&self) -> &str {
        "enter"
    }

    fn signature(&self) -> registry::Signature {
        Signature::build("enter").required("location", SyntaxShape::Path)
    }

    fn usage(&self) -> &str {
        "Create a new shell and begin at this path."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &registry::CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let raw_args = raw_args.clone();
        match call_info.args.expect_nth(0)? {
            Tagged {
                item: Value::Primitive(Primitive::Path(location)),
                ..
            } => {
                let location_string = location.display().to_string();
                let location_clone = location_string.clone();

                if location.starts_with("help") {
                    let spec = location_string.split(":").collect::<Vec<&str>>();

                    let (_, command) = (spec[0], spec[1]);

                    if registry.has(command) {
                        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                            Value::string(command).tagged(Tag::unknown()),
                        )))]
                        .into())
                    } else {
                        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::EnterHelpShell(
                            Value::nothing().tagged(Tag::unknown()),
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

                        let full_path = std::path::PathBuf::from(cwd);

                        let (file_extension, contents, contents_tag, anchor_location) =
                            crate::commands::open::fetch(
                                &full_path,
                                &location_clone,
                                Span::unknown(),
                            )
                            .await.unwrap();

                        if contents_tag.anchor != uuid::Uuid::nil() {
                            // If we have loaded something, track its source
                            yield ReturnSuccess::action(CommandAction::AddAnchorLocation(
                                contents_tag.anchor,
                                anchor_location,
                            ));
                        }


                        match contents {
                            Value::Primitive(Primitive::String(_)) => {
                                let tagged_contents = contents.tagged(contents_tag);

                                if let Some(extension) = file_extension {
                                    let command_name = format!("from-{}", extension);
                                    if let Some(converter) =
                                        registry.get_command(&command_name)
                                    {
                                        let new_args = RawCommandArgs {
                                            host: raw_args.host,
                                            shell_manager: raw_args.shell_manager,
                                            call_info: UnevaluatedCallInfo {
                                                args: crate::parser::hir::Call {
                                                    head: raw_args.call_info.args.head,
                                                    positional: None,
                                                    named: None,
                                                },
                                                source: raw_args.call_info.source,
                                                source_map: raw_args.call_info.source_map,
                                                name_tag: raw_args.call_info.name_tag,
                                            },
                                        };
                                        let mut result = converter.run(
                                            new_args.with_input(vec![tagged_contents]),
                                            &registry,
                                            false
                                        );
                                        let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                                            result.drain_vec().await;
                                        for res in result_vec {
                                            match res {
                                                Ok(ReturnSuccess::Value(Tagged {
                                                    item,
                                                    ..
                                                })) => {
                                                    yield Ok(ReturnSuccess::Action(CommandAction::EnterValueShell(
                                                        Tagged {
                                                            item,
                                                            tag: contents_tag,
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
                                let tagged_contents = contents.tagged(contents_tag);

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
