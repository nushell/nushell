use crate::parser::hir;
use crate::prelude::*;
use derive_new::new;
use log::{log_enabled, trace};

use super::ClassifiedInputStream;

#[derive(new, Debug, Clone, Eq, PartialEq)]
pub struct Command {
    pub(crate) name: String,
    pub(crate) name_tag: Tag,
    pub(crate) args: hir::Call,
}

impl HasSpan for Command {
    fn span(&self) -> Span {
        let start = self.name_tag.span;

        start.until(self.args.span)
    }
}

impl PrettyDebugWithSource for Command {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "internal command",
            b::description(&self.name) + b::space() + self.args.pretty_debug(source),
        )
    }
}

impl Command {
    pub(crate) fn run(
        self,
        context: &mut Context,
        input: ClassifiedInputStream,
        source: Text,
    ) -> Result<InputStream, ShellError> {
        if log_enabled!(log::Level::Trace) {
            trace!(target: "nu::run::internal", "->");
            trace!(target: "nu::run::internal", "{}", self.name);
            trace!(target: "nu::run::internal", "{}", self.args.debug(&source));
        }

        let objects: InputStream =
            trace_stream!(target: "nu::trace_stream::internal", "input" = input.objects);

        let command = context.expect_command(&self.name);

        let result =
            { context.run_command(command, self.name_tag.clone(), self.args, &source, objects) };

        let result = trace_out_stream!(target: "nu::trace_stream::internal", "output" = result);
        let mut result = result.values;
        let mut context = context.clone();

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
                                        ).unwrap(),
                                    ));
                                }
                                _ => {
                                    context.shell_manager.insert_at_current(Box::new(
                                        HelpShell::index(&context.registry()).unwrap(),
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
                                FilesystemShell::with_location(location, context.registry().clone()).unwrap(),
                            ));
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

                    Ok(ReturnSuccess::Value(v)) => {
                        yielded = true;
                        yield Ok(v);
                    }

                    Ok(ReturnSuccess::DebugValue(v)) => {
                        yielded = true;

                        let doc = PrettyDebug::pretty_doc(&v);
                        let mut buffer = termcolor::Buffer::ansi();

                        doc.render_raw(
                            context.with_host(|host| host.width() - 5),
                            &mut crate::parser::debug::TermColored::new(&mut buffer),
                        ).unwrap();

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
}
