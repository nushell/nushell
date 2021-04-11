use crate::call_info::UnevaluatedCallInfo;
use crate::command_args::RawCommandArgs;
use crate::evaluation_context::EvaluationContext;
use crate::filesystem::filesystem_shell::{FilesystemShell, FilesystemShellMode};
use crate::shell::value_shell::ValueShell;
use log::{log_enabled, trace};
use nu_errors::ShellError;
use nu_protocol::hir::{ExternalRedirection, InternalCommand};
use nu_protocol::{CommandAction, ReturnSuccess, UntaggedValue, Value};
use nu_source::{PrettyDebug, Span, Tag};
use nu_stream::{InputStream, OutputStream};

pub(crate) fn run_internal_command(
    command: InternalCommand,
    context: &EvaluationContext,
    input: InputStream,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::internal", "->");
        trace!(target: "nu::run::internal", "{}", command.name);
    }

    let objects: InputStream = input;

    let internal_command = context.scope.expect_command(&command.name);

    let result = {
        context.run_command(
            internal_command?,
            Tag::unknown_anchor(command.name_span),
            command.args.clone(),
            objects,
        )?
    };

    Ok(InputStream::from_stream(
        InternalIterator {
            command,
            context: context.clone(),
            leftovers: vec![],
            input: result,
        }
        .take_while(|x| !x.is_error()),
    ))
}

struct InternalIterator {
    context: EvaluationContext,
    command: InternalCommand,
    leftovers: Vec<Value>,
    input: OutputStream,
}

impl Iterator for InternalIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        // let head = Arc::new(command.args.head.clone());
        // let context = context.clone();
        // let command = Arc::new(command);

        if !self.leftovers.is_empty() {
            let output = self.leftovers.remove(0);
            return Some(output);
        }

        while let Some(item) = self.input.next() {
            match item {
                Ok(ReturnSuccess::Action(action)) => match action {
                    CommandAction::ChangePath(path) => {
                        self.context.shell_manager.set_path(path);
                    }
                    CommandAction::Exit(code) => std::process::exit(code), // TODO: save history.txt
                    CommandAction::Error(err) => {
                        self.context.error(err);
                    }
                    CommandAction::AutoConvert(tagged_contents, extension) => {
                        let contents_tag = tagged_contents.tag.clone();
                        let command_name = format!("from {}", extension);
                        if let Some(converter) = self.context.scope.get_command(&command_name) {
                            let new_args = RawCommandArgs {
                                host: self.context.host.clone(),
                                ctrl_c: self.context.ctrl_c.clone(),
                                configs: self.context.configs.clone(),
                                current_errors: self.context.current_errors.clone(),
                                shell_manager: self.context.shell_manager.clone(),
                                call_info: UnevaluatedCallInfo {
                                    args: nu_protocol::hir::Call {
                                        head: self.command.args.head.clone(),
                                        positional: None,
                                        named: None,
                                        span: Span::unknown(),
                                        external_redirection: ExternalRedirection::Stdout,
                                    },
                                    name_tag: Tag::unknown_anchor(self.command.name_span),
                                },
                                scope: self.context.scope.clone(),
                            };
                            let result = converter.run(new_args.with_input(vec![tagged_contents]));

                            match result {
                                Ok(mut result) => {
                                    let result_vec: Vec<Result<ReturnSuccess, ShellError>> =
                                        result.drain_vec();

                                    let mut output = vec![];
                                    for res in result_vec {
                                        match res {
                                            Ok(ReturnSuccess::Value(Value {
                                                value: UntaggedValue::Table(list),
                                                ..
                                            })) => {
                                                for l in list {
                                                    output.push(l);
                                                }
                                            }
                                            Ok(ReturnSuccess::Value(Value { value, .. })) => {
                                                output.push(value.into_value(contents_tag.clone()));
                                            }
                                            Err(e) => output.push(
                                                UntaggedValue::Error(e).into_untagged_value(),
                                            ),
                                            _ => {}
                                        }
                                    }

                                    let mut output = output.into_iter();

                                    if let Some(x) = output.next() {
                                        self.leftovers = output.collect();

                                        return Some(x);
                                    }
                                }
                                Err(err) => {
                                    self.context.error(err);
                                }
                            }
                        } else {
                            return Some(tagged_contents);
                        }
                    }
                    CommandAction::EnterValueShell(value) => {
                        self.context
                            .shell_manager
                            .insert_at_current(Box::new(ValueShell::new(value)));
                    }
                    CommandAction::EnterShell(location) => {
                        let mode = if self.context.shell_manager.is_interactive() {
                            FilesystemShellMode::Cli
                        } else {
                            FilesystemShellMode::Script
                        };
                        self.context.shell_manager.insert_at_current(Box::new(
                            match FilesystemShell::with_location(location, mode) {
                                Ok(v) => v,
                                Err(err) => {
                                    self.context.error(err.into());
                                    break;
                                }
                            },
                        ));
                    }
                    CommandAction::AddPlugins(path) => {
                        match crate::plugin::build_plugin::scan(vec![std::path::PathBuf::from(
                            path,
                        )]) {
                            Ok(plugins) => {
                                self.context.add_commands(
                                    plugins
                                        .into_iter()
                                        .filter(|p| !self.context.is_command_registered(p.name()))
                                        .collect(),
                                );
                            }
                            Err(reason) => {
                                self.context.error(reason);
                            }
                        }
                    }
                    CommandAction::PreviousShell => {
                        self.context.shell_manager.prev();
                    }
                    CommandAction::NextShell => {
                        self.context.shell_manager.next();
                    }
                    CommandAction::LeaveShell(code) => {
                        self.context.shell_manager.remove_at_current();
                        if self.context.shell_manager.is_empty() {
                            std::process::exit(code); // TODO: save history.txt
                        }
                    }
                    CommandAction::UnloadConfig(cfg_path) => {
                        self.context.unload_config(&cfg_path);
                    }
                    CommandAction::LoadConfig(cfg_path) => {
                        if let Err(e) = self.context.load_config(&cfg_path) {
                            return Some(UntaggedValue::Error(e).into_untagged_value());
                        }
                    }
                },

                Ok(ReturnSuccess::Value(Value {
                    value: UntaggedValue::Error(err),
                    ..
                })) => {
                    self.context.error(err);
                }

                Ok(ReturnSuccess::Value(v)) => return Some(v),

                Ok(ReturnSuccess::DebugValue(v)) => {
                    let doc = PrettyDebug::pretty_doc(&v);
                    let mut buffer = termcolor::Buffer::ansi();

                    let _ = doc.render_raw(
                        self.context.with_host(|host| host.width() - 5),
                        &mut nu_source::TermColored::new(&mut buffer),
                    );

                    let value = String::from_utf8_lossy(buffer.as_slice());

                    return Some(UntaggedValue::string(value).into_untagged_value());
                }

                Err(err) => {
                    self.context.error(err);
                }
            }
        }

        None
    }
}
