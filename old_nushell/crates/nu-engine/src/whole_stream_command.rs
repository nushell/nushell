use crate::command_args::CommandArgs;
use crate::documentation::get_full_help;
use crate::evaluate::block::run_block;
use crate::example::Example;
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::hir::Block;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::{DbgDocBldr, DebugDocBuilder, PrettyDebugWithSource, Span, Tag};
use nu_stream::{ActionStream, InputStream, IntoOutputStream, OutputStream};
use std::sync::Arc;

pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature::new(self.name()).desc(self.usage()).filter()
    }

    fn usage(&self) -> &str;

    fn extra_usage(&self) -> &str {
        ""
    }

    fn run_with_actions(&self, _args: CommandArgs) -> Result<ActionStream, ShellError> {
        return Err(ShellError::unimplemented(&format!(
            "{} does not implement run or run_with_actions",
            self.name()
        )));
    }

    fn run(&self, args: CommandArgs) -> Result<InputStream, ShellError> {
        let context = args.context.clone();
        let stream = self.run_with_actions(args)?;

        Ok(Box::new(crate::evaluate::internal::InternalIterator {
            context,
            input: stream,
            leftovers: InputStream::empty(),
        })
        .into_output_stream())
    }

    fn is_binary(&self) -> bool {
        false
    }

    // Commands that are not meant to be run by users
    fn is_private(&self) -> bool {
        false
    }

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }

    // This is a built-in command
    fn is_builtin(&self) -> bool {
        true
    }

    // Is a sub command
    fn is_sub(&self) -> bool {
        self.name().contains(' ')
    }

    // Is a plugin command
    fn is_plugin(&self) -> bool {
        false
    }

    // Is a custom command i.e. def blah [] { }
    fn is_custom(&self) -> bool {
        false
    }
}

// Custom commands are blocks, so we can use the information in the block to also
// implement a WholeStreamCommand
#[allow(clippy::suspicious_else_formatting)]
impl WholeStreamCommand for Arc<Block> {
    fn name(&self) -> &str {
        &self.params.name
    }

    fn signature(&self) -> Signature {
        self.params.clone()
    }

    fn usage(&self) -> &str {
        &self.params.usage
    }

    fn extra_usage(&self) -> &str {
        &self.params.extra_usage
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let call_info = args.call_info.clone();

        let block = self.clone();

        let external_redirection = args.call_info.args.external_redirection;

        let ctx = &args.context;
        let evaluated = call_info.evaluate(ctx)?;

        let input = args.input;
        ctx.scope.enter_scope();
        if let Some(args) = evaluated.args.positional {
            let mut args_iter = args.into_iter().peekable();
            let mut params_iter = self.params.positional.iter();
            loop {
                match (args_iter.peek(), params_iter.next()) {
                    (Some(_), Some(param)) => {
                        let name = param.0.name();
                        // we just checked the peek above, so this should be infallible
                        if let Some(arg) = args_iter.next() {
                            if name.starts_with('$') {
                                ctx.scope.add_var(name.to_string(), arg);
                            } else {
                                ctx.scope.add_var(format!("${}", name), arg);
                            }
                        }
                    }
                    (Some(arg), None) => {
                        if block.params.rest_positional.is_none() {
                            ctx.scope.exit_scope();
                            return Err(ShellError::labeled_error(
                                "Unexpected argument to command",
                                "unexpected argument",
                                arg.tag.span,
                            ));
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            if let Some(rest_pos) = &block.params.rest_positional {
                let elements: Vec<_> = args_iter.collect();
                let start = if let Some(first) = elements.first() {
                    first.tag.span.start()
                } else {
                    0
                };
                let end = if let Some(last) = elements.last() {
                    last.tag.span.end()
                } else {
                    0
                };

                ctx.scope.add_var(
                    format!("${}", rest_pos.0),
                    UntaggedValue::Table(elements).into_value(Span::new(start, end)),
                );
            }
        } else if let Some(rest_pos) = &block.params.rest_positional {
            //If there is a rest arg, but no args were provided,
            //we have to set $rest to an empty table
            ctx.scope.add_var(
                format!("${}", rest_pos.0),
                UntaggedValue::Table(Vec::new()).into_value(Span::new(0, 0)),
            );
        }
        if let Some(args) = evaluated.args.named {
            for named in &block.params.named {
                let name = named.0;
                if let Some(value) = args.get(name) {
                    if name.starts_with('$') {
                        ctx.scope.add_var(name, value.clone());
                    } else {
                        ctx.scope.add_var(format!("${}", name), value.clone());
                    }
                } else if name.starts_with('$') {
                    ctx.scope
                        .add_var(name, UntaggedValue::nothing().into_untagged_value());
                } else {
                    ctx.scope.add_var(
                        format!("${}", name),
                        UntaggedValue::nothing().into_untagged_value(),
                    );
                }
            }
        } else {
            for named in &block.params.named {
                let name = named.0;
                if name.starts_with('$') {
                    ctx.scope
                        .add_var(name, UntaggedValue::nothing().into_untagged_value());
                } else {
                    ctx.scope.add_var(
                        format!("${}", name),
                        UntaggedValue::nothing().into_untagged_value(),
                    );
                }
            }
        }
        let result = run_block(&block, ctx, input, external_redirection);
        ctx.scope.exit_scope();
        result
    }

    fn is_binary(&self) -> bool {
        false
    }

    fn is_private(&self) -> bool {
        false
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn is_custom(&self) -> bool {
        true
    }

    fn is_builtin(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct Command(Arc<dyn WholeStreamCommand>);

impl PrettyDebugWithSource for Command {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::typed(
            "whole stream command",
            DbgDocBldr::description(self.name())
                + DbgDocBldr::space()
                + DbgDocBldr::equals()
                + DbgDocBldr::space()
                + self.signature().pretty_debug(source),
        )
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command({})", self.name())
    }
}

impl Command {
    pub fn name(&self) -> &str {
        self.0.name()
    }

    pub fn signature(&self) -> Signature {
        self.0.signature()
    }

    pub fn usage(&self) -> &str {
        self.0.usage()
    }

    pub fn extra_usage(&self) -> &str {
        self.0.extra_usage()
    }

    pub fn examples(&self) -> Vec<Example> {
        self.0.examples()
    }

    pub fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        if args.call_info.switch_present("help") {
            let cl = self.0.clone();
            Ok(ActionStream::one(Ok(ReturnSuccess::Value(
                UntaggedValue::string(get_full_help(&*cl, &args.context.scope))
                    .into_value(Tag::unknown()),
            ))))
        } else {
            self.0.run_with_actions(args)
        }
    }

    pub fn run(&self, args: CommandArgs) -> Result<InputStream, ShellError> {
        if args.call_info.switch_present("help") {
            let cl = self.0.clone();
            Ok(InputStream::one(
                UntaggedValue::string(get_full_help(&*cl, &args.context.scope))
                    .into_value(Tag::unknown()),
            ))
        } else {
            self.0.run(args)
        }
    }

    pub fn is_binary(&self) -> bool {
        self.0.is_binary()
    }

    pub fn is_private(&self) -> bool {
        self.0.is_private()
    }

    pub fn stream_command(&self) -> &dyn WholeStreamCommand {
        &*self.0
    }

    pub fn is_builtin(&self) -> bool {
        self.0.is_builtin()
    }

    pub fn is_sub(&self) -> bool {
        self.0.is_sub()
    }

    pub fn is_plugin(&self) -> bool {
        self.0.is_plugin()
    }

    pub fn is_custom(&self) -> bool {
        self.0.is_custom()
    }
}

pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Command {
    Command(Arc::new(command))
}
