use crate::commands::classified::external;
use crate::prelude::*;

use derive_new::new;
use parking_lot::Mutex;

use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::hir::{Expression, ExternalArgs, ExternalCommand, Literal, SpannedExpression};
use nu_protocol::{Signature, SyntaxShape};

#[derive(Deserialize)]
pub struct RunExternalArgs {}

#[derive(new)]
pub struct RunExternalCommand;

fn spanned_expression_to_string(expr: SpannedExpression) -> Result<String, ShellError> {
    if let SpannedExpression {
        expr: Expression::Literal(Literal::String(s)),
        ..
    } = expr
    {
        Ok(s)
    } else {
        Err(ShellError::labeled_error(
            "Expected string for command name",
            "expected string",
            expr.span,
        ))
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
        "Runs external command (not a nushell builtin)"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Run the external echo command",
            example: "run_external echo 'nushell'",
            result: None,
        }]
    }

    fn is_internal(&self) -> bool {
        true
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let positionals = args.call_info.args.positional.clone().ok_or_else(|| {
            ShellError::untagged_runtime_error("positional arguments unexpectedly empty")
        })?;

        let mut positionals = positionals.into_iter();

        let external_redirection = args.call_info.args.external_redirection;

        let name = positionals
            .next()
            .ok_or_else(|| {
                ShellError::untagged_runtime_error("run_external called with no arguments")
            })
            .and_then(spanned_expression_to_string)?;

        let mut external_context = {
            EvaluationContext {
                scope: args.scope.clone(),
                host: args.host.clone(),
                shell_manager: args.shell_manager.clone(),
                ctrl_c: args.ctrl_c.clone(),
                configs: args.configs.clone(),
                current_errors: Arc::new(Mutex::new(vec![])),
            }
        };

        let command = ExternalCommand {
            name,
            name_tag: args.call_info.name_tag.clone(),
            args: ExternalArgs {
                list: positionals.collect(),
                span: args.call_info.args.span,
            },
        };

        let input = args.input;
        let result = external::run_external_command(
            command,
            &mut external_context,
            input,
            external_redirection,
        );

        Ok(result?.to_output_stream())
    }
}
