use crate::commands::classified::external;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use parking_lot::Mutex;

use nu_errors::ShellError;
use nu_protocol::hir::{
    Expression, ExternalArg, ExternalArgs, ExternalCommand, Literal, SpannedExpression,
};
use nu_protocol::{ReturnSuccess, Scope, Signature, SyntaxShape};

#[derive(Deserialize)]
pub struct RunExternalArgs {}

#[derive(new)]
pub struct RunExternalCommand;

fn spanned_expression_to_string(expr: &SpannedExpression) -> String {
    if let SpannedExpression {
        expr: Expression::Literal(Literal::String(s)),
        ..
    } = expr
    {
        s.clone()
    } else {
        "notacommand!!!".to_string()
    }
}

impl WholeStreamCommand for RunExternalCommand {
    fn name(&self) -> &str {
        "run_external"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).rest(SyntaxShape::Any, "external command arguments")
    }

    fn usage(&self) -> &str {
        ""
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let positionals = args.call_info.args.positional.ok_or_else(|| {
            ShellError::untagged_runtime_error("positional arguments unexpectedly empty")
        })?;

        let mut command_args = positionals.iter();
        let name = command_args
            .next()
            .map(spanned_expression_to_string)
            .ok_or_else(|| {
                ShellError::untagged_runtime_error(
                    "run_external unexpectedly missing external name positional arg",
                )
            })?;

        let command = ExternalCommand {
            name,
            name_tag: args.call_info.name_tag.clone(),
            args: ExternalArgs {
                list: command_args
                    .map(|arg| ExternalArg {
                        arg: spanned_expression_to_string(arg),
                        tag: Tag::unknown_anchor(arg.span),
                    })
                    .collect(),
                span: args.call_info.args.span,
            },
        };

        let mut external_context;
        #[cfg(windows)]
        {
            external_context = Context {
                registry: registry.clone(),
                host: args.host.clone(),
                shell_manager: args.shell_manager.clone(),
                ctrl_c: args.ctrl_c.clone(),
                current_errors: Arc::new(Mutex::new(vec![])),
                windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
            };
        }
        #[cfg(not(windows))]
        {
            external_context = Context {
                registry: registry.clone(),
                host: args.host.clone(),
                shell_manager: args.shell_manager.clone(),
                ctrl_c: args.ctrl_c.clone(),
                current_errors: Arc::new(Mutex::new(vec![])),
            };
        }

        let is_last = args.call_info.args.is_last;
        let input = args.input;
        let stream = async_stream! {
            let scope = Scope::empty();
            let result = external::run_external_command(
                command,
                &mut external_context,
                input,
                &scope,
                is_last,
            ).await;

            match result {
                Ok(mut stream) => {
                    while let Some(value) = stream.next().await {
                        yield Ok(ReturnSuccess::Value(value));
                    }
                },
                Err(e) => {
                    yield Err(e);
                },
                _ => {}
            }
        };

        Ok(stream.to_output_stream())
    }
}
