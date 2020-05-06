use crate::commands::command::whole_stream_command;
use crate::commands::run_alias::AliasCommand;
use crate::commands::UnevaluatedCallInfo;
use crate::prelude::*;
use log::{log_enabled, trace};
use nu_errors::ShellError;
use nu_protocol::hir::InternalCommand;
use nu_protocol::{CommandAction, Primitive, ReturnSuccess, Scope, UntaggedValue, Value};

pub(crate) fn run_internal_command(
    command: InternalCommand,
    context: &mut Context,
    input: InputStream,
    scope: &Scope,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::internal", "->");
        trace!(target: "nu::run::internal", "{}", command.name);
    }

    let objects: InputStream = trace_stream!(target: "nu::trace_stream::internal", "input" = input);
    let internal_command = context.expect_command(&command.name);

    let result = {
        context.run_command(
            internal_command?,
            Tag::unknown_anchor(command.name_span),
            command.args.clone(),
            scope,
            objects,
        )
    };

    let mut result = trace_out_stream!(target: "nu::trace_stream::internal", "output" = result);
    let mut context = context.clone();
    let scope = scope.clone();

    let stream = async_stream! {
        let mut soft_errs: Vec<ShellError> = vec![];
        let mut yielded = false;

        while let Some(item) = result.next().await {
            match item {
                Ok(ReturnSuccess::Action(action)) => match action {
                    CommandAction::ChangePath(path) => {
                        context.shell_manager.set_path(path);
                    }
                    CommandAction::Exit => std::process::exit(0), // TODO: save history.txt
                    CommandAction::Error(err) => {
                        context.error(err);
                        break;
                    }
                    CommandAction::AutoConvert(tagged_contents, extension) => {
                        let contents_tag = tagged_contents.tag.clone();
                        let command_name = format!("from {}", extension);
                        let command = command.clone();
                        if let Some(converter) = context.registry.get_command(&command_name) {
                            let new_args = RawCommandArgs {
                                host: context.host.clone(),
                                ctrl_c: context.ctrl_c.clone(),
                                shell_manager: context.shell_manager.clone(),
                                call_info: UnevaluatedCallInfo {
                                    args: nu_protocol::hir::Call {
                                        head: command.args.head,
                                        positional: None,
                                        named: None,
                                        span: Span::unknown(),
                                        is_last: false,
                                    },
                                    name_tag: Tag::unknown_anchor(command.name_span),
                                    scope: scope.clone(),
                                }
                            };
                            let mut result = converter.run(new_args.with_input(vec![tagged_contents]), &context.registry);
                            let result_vec: Vec<Result<ReturnSuccess, ShellError>> = result.drain_vec().await;
                            for res in result_vec {
                                match res {
                                    Ok(ReturnSuccess::Value(Value { value: UntaggedValue::Table(list), ..})) => {
                                        for l in list {
                                            yield Ok(l);
                                        }
                                    }
                                    Ok(ReturnSuccess::Value(Value { value, .. })) => {
                                        yield Ok(value.into_value(contents_tag.clone()));
                                    }
                                    Err(e) => yield Err(e),
                                    _ => {}
                                }
                            }
                        } else {
                            yield Ok(tagged_contents)
                        }
                    }
                    CommandAction::EnterHelpShell(value) => {
                        match value {
                            Value {
                                value: UntaggedValue::Primitive(Primitive::String(cmd)),
                                tag,
                            } => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::for_command(
                                        UntaggedValue::string(cmd).into_value(tag),
                                        &context.registry(),
                                    )?,
                                ));
                            }
                            _ => {
                                context.shell_manager.insert_at_current(Box::new(
                                    HelpShell::index(&context.registry())?,
                                ));
                            }
                        }
                    }
                    CommandAction::EnterValueShell(value) => {
                        context
                            .shell_manager
                            .insert_at_current(Box::new(ValueShell::new(value)));
                    }
                    CommandAction::EnterShell(location) => {
                        context.shell_manager.insert_at_current(Box::new(
                            FilesystemShell::with_location(location, context.registry().clone())?,
                        ));
                    }
                    CommandAction::AddAlias(name, args, block) => {
                        context.add_commands(vec![
                            whole_stream_command(AliasCommand::new(
                                name,
                                args,
                                block,
                            ))
                        ]);
                    }
                    CommandAction::PreviousShell => {
                        context.shell_manager.prev();
                    }
                    CommandAction::NextShell => {
                        context.shell_manager.next();
                    }
                    CommandAction::LeaveShell => {
                        context.shell_manager.remove_at_current();
                        if context.shell_manager.is_empty() {
                            std::process::exit(0); // TODO: save history.txt
                        }
                    }
                },

                Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Error(err),
                    ..
                })) => {
                    context.error(err.clone());
                    yield Err(err);
                    break;
                }

                Ok(ReturnSuccess::Value(v)) => {
                    yielded = true;
                    yield Ok(v);
                }

                Ok(ReturnSuccess::DebugValue(v)) => {
                    yielded = true;

                    let doc = PrettyDebug::pretty_doc(&v);
                    let mut buffer = termcolor::Buffer::ansi();

                    let _ = doc.render_raw(
                        context.with_host(|host| host.width() - 5),
                        &mut nu_source::TermColored::new(&mut buffer),
                    );

                    let value = String::from_utf8_lossy(buffer.as_slice());

                    yield Ok(UntaggedValue::string(value).into_untagged_value())
                }

                Err(err) => {
                    context.error(err);
                    break;
                }
            }
        }
    };

    Ok(stream.to_input_stream())
}
