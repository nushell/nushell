use crate::call_info::UnevaluatedCallInfo;
use crate::evaluation_context::EvaluationContext;
use crate::filesystem::filesystem_shell::{FilesystemShell, FilesystemShellMode};
use crate::shell::value_shell::ValueShell;
use crate::CommandArgs;
use log::{log_enabled, trace};
use nu_errors::ShellError;
use nu_protocol::hir::{
    Expression, ExternalRedirection, InternalCommand, SpannedExpression, Synthetic,
};
use nu_protocol::{CommandAction, ReturnSuccess, UntaggedValue, Value};
use nu_source::{PrettyDebug, Span, Tag};
use nu_stream::{ActionStream, InputStream};

pub(crate) fn run_internal_command(
    command: &InternalCommand,
    context: &EvaluationContext,
    input: InputStream,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::internal", "->");
        trace!(target: "nu::run::internal", "{}", command.name);
    }

    let objects: InputStream = input;

    let internal_command = context.scope.expect_command(&command.name);
    let internal_command = internal_command?;

    let result = context.run_command(
        internal_command,
        Tag::unknown_anchor(command.name_span),
        command.args.clone(), // FIXME: this is inefficient
        objects,
    )?;
    Ok(InputStream::from_stream(InternalIteratorSimple {
        context: context.clone(),
        input: result,
    }))
}

struct InternalIteratorSimple {
    context: EvaluationContext,
    input: InputStream,
}

impl Iterator for InternalIteratorSimple {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.next() {
            Some(Value {
                value: UntaggedValue::Error(err),
                ..
            }) => {
                self.context.error(err);
                None
            }
            x => x,
        }
    }
}

pub struct InternalIterator {
    pub context: EvaluationContext,
    pub leftovers: InputStream,
    pub input: ActionStream,
}

impl Iterator for InternalIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(output) = self.leftovers.next() {
            return Some(output);
        }

        while let Some(item) = self.input.next() {
            match item {
                Ok(ReturnSuccess::Action(action)) => match action {
                    CommandAction::ChangePath(path) => {
                        self.context.shell_manager().set_path(path);
                    }
                    CommandAction::Exit(code) => std::process::exit(code), // TODO: save history.txt
                    CommandAction::Error(err) => {
                        self.context.error(err);
                        return None;
                    }
                    CommandAction::AutoConvert(tagged_contents, extension) => {
                        let contents_tag = tagged_contents.tag.clone();
                        let command_name = format!("from {}", extension);
                        if let Some(converter) = self.context.scope.get_command(&command_name) {
                            let new_args = CommandArgs {
                                context: self.context.clone(),
                                call_info: UnevaluatedCallInfo {
                                    args: nu_protocol::hir::Call {
                                        head: Box::new(SpannedExpression {
                                            expr: Expression::Synthetic(Synthetic::String(
                                                command_name.clone(),
                                            )),
                                            span: tagged_contents.tag().span,
                                        }),
                                        positional: None,
                                        named: None,
                                        span: Span::unknown(),
                                        external_redirection: ExternalRedirection::Stdout,
                                    },
                                    name_tag: tagged_contents.tag(),
                                },
                                input: InputStream::one(tagged_contents),
                            };
                            let result = converter.run(new_args);

                            match result {
                                Ok(mut result) => {
                                    if let Some(x) = result.next() {
                                        self.leftovers =
                                            InputStream::from_stream(result.map(move |x| Value {
                                                value: x.value,
                                                tag: contents_tag.clone(),
                                            }));
                                        return Some(x);
                                    } else {
                                        return None;
                                    }
                                }
                                Err(err) => {
                                    self.leftovers = InputStream::empty();
                                    return Some(Value::error(err));
                                }
                            }
                        } else {
                            return Some(tagged_contents);
                        }
                    }
                    CommandAction::EnterValueShell(value) => {
                        self.context
                            .shell_manager()
                            .insert_at_current(Box::new(ValueShell::new(value)));
                    }
                    CommandAction::EnterShell(location) => {
                        let mode = if self.context.shell_manager().is_interactive() {
                            FilesystemShellMode::Cli
                        } else {
                            FilesystemShellMode::Script
                        };
                        self.context.shell_manager().insert_at_current(Box::new(
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
                        self.context.shell_manager().prev();
                    }
                    CommandAction::NextShell => {
                        self.context.shell_manager().next();
                    }
                    CommandAction::GotoShell(i) => {
                        self.context.shell_manager().goto(i);
                    }
                    CommandAction::LeaveShell(code) => {
                        self.context.shell_manager().remove_at_current();
                        if self.context.shell_manager().is_empty() {
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
                    return None;
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
