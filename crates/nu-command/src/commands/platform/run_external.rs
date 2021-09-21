use crate::classified::external;
use crate::prelude::*;

use derive_new::new;
use std::path::PathBuf;

use nu_engine::WholeStreamCommand;
use nu_engine::{evaluate_baseline_expr, shell::CdArgs};
use nu_errors::ShellError;
use nu_path::{canonicalize, trim_trailing_slash};
use nu_protocol::{
    hir::{ExternalArgs, ExternalCommand, SpannedExpression},
    Primitive, UntaggedValue,
};
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(new)]
pub struct RunExternalCommand {
    /// Whether or not nushell is being used in an interactive context
    pub(crate) interactive: bool,
}

fn spanned_expression_to_string(
    expr: SpannedExpression,
    ctx: &EvaluationContext,
) -> Result<String, ShellError> {
    let value = evaluate_baseline_expr(&expr, ctx)?;

    if let UntaggedValue::Primitive(Primitive::String(s)) = value.value {
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
        Signature::build(self.name()).rest("rest", SyntaxShape::Any, "external command arguments")
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

    fn is_private(&self) -> bool {
        true
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let positionals = args.call_info.args.positional.clone().ok_or_else(|| {
            ShellError::untagged_runtime_error("positional arguments unexpectedly empty")
        })?;

        let mut positionals = positionals.into_iter();

        let external_redirection = args.call_info.args.external_redirection;

        let expr = positionals.next().ok_or_else(|| {
            ShellError::untagged_runtime_error("run_external called with no arguments")
        })?;

        let name = spanned_expression_to_string(expr, &args.context)?;

        let mut external_context = args.context.clone();

        let is_interactive = self.interactive;

        let command = ExternalCommand {
            name,
            name_tag: args.call_info.name_tag.clone(),
            args: ExternalArgs {
                list: positionals.collect(),
                span: args.call_info.args.span,
            },
        };

        // If we're in interactive mode, we will "auto cd". That is, instead of interpreting
        // this as an external command, we will see it as a path and `cd` into it.
        if is_interactive {
            if let Some(path) = maybe_autocd_dir(&command, &mut external_context) {
                let cd_args = CdArgs {
                    path: Some(Tagged {
                        item: PathBuf::from(path),
                        tag: args.call_info.name_tag.clone(),
                    }),
                };

                let result = external_context
                    .shell_manager()
                    .cd(cd_args, args.call_info.name_tag);

                return Ok(result?.into_action_stream());
            }
        }

        let input = args.input;
        let result = external::run_external_command(
            command,
            &mut external_context,
            input,
            external_redirection,
        );

        // When externals return, don't let them mess up the ansi escapes
        #[cfg(windows)]
        {
            let _ = nu_ansi_term::enable_ansi_support();
        }

        Ok(result?.into_action_stream())
    }
}

#[allow(unused_variables)]
fn maybe_autocd_dir(cmd: &ExternalCommand, ctx: &mut EvaluationContext) -> Option<String> {
    // We will "auto cd" if
    //   - the command name ends in a path separator, or
    //   - it's not a command on the path and no arguments were given.
    let name = &cmd.name;
    ctx.sync_path_to_env();
    let path_name = if name.ends_with(std::path::is_separator)
        || (cmd.args.is_empty()
            && PathBuf::from(name).is_dir()
            && canonicalize(name).is_ok()
            && !ctx.host().lock().is_external_cmd(name))
    {
        Some(trim_trailing_slash(name))
    } else {
        None
    };

    path_name.map(|name| {
        #[cfg(windows)]
        {
            if name.ends_with(':') {
                // This looks like a drive shortcut. We need to a) switch drives and b) go back to the previous directory we were viewing on that drive
                // But first, we need to save where we are now
                let current_path = ctx.shell_manager().path();

                let split_path: Vec<_> = current_path.split(':').collect();
                if split_path.len() > 1 {
                    ctx.windows_drives_previous_cwd()
                        .lock()
                        .insert(split_path[0].to_string(), current_path);
                }

                let name = name.to_uppercase();
                let new_drive: Vec<_> = name.split(':').collect();

                if let Some(val) = ctx.windows_drives_previous_cwd().lock().get(new_drive[0]) {
                    val.to_string()
                } else {
                    name
                }
            } else {
                name.to_string()
            }
        }
        #[cfg(not(windows))]
        {
            name.to_string()
        }
    })
}
