use crate::commands::cd::CdArgs;
use crate::commands::classified::external;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use derive_new::new;
use parking_lot::Mutex;
use std::path::PathBuf;

use nu_errors::ShellError;
use nu_protocol::hir::{Expression, ExternalArgs, ExternalCommand, Literal, SpannedExpression};
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
pub struct RunExternalArgs {}

#[derive(new)]
pub struct RunExternalCommand {
    /// Whether or not nushell is being used in an interactive context
    pub(crate) interactive: bool,
}

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

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
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
            #[cfg(windows)]
            {
                Context {
                    registry: registry.clone(),
                    host: args.host.clone(),
                    user_recently_used_autoenv_untrust: false,
                    shell_manager: args.shell_manager.clone(),
                    ctrl_c: args.ctrl_c.clone(),
                    current_errors: Arc::new(Mutex::new(vec![])),
                    windows_drives_previous_cwd: Arc::new(Mutex::new(
                        std::collections::HashMap::new(),
                    )),
                    raw_input: String::default(),
                }
            }
            #[cfg(not(windows))]
            {
                Context {
                    registry: registry.clone(),
                    user_recently_used_autoenv_untrust: false,
                    host: args.host.clone(),
                    shell_manager: args.shell_manager.clone(),
                    ctrl_c: args.ctrl_c.clone(),
                    current_errors: Arc::new(Mutex::new(vec![])),
                    raw_input: String::default(),
                }
            }
        };

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
            if let Some(path) = maybe_autocd_dir(&command, &mut external_context).await {
                let cd_args = CdArgs {
                    path: Some(Tagged {
                        item: PathBuf::from(path),
                        tag: args.call_info.name_tag.clone(),
                    }),
                };

                let result = external_context
                    .shell_manager
                    .cd(cd_args, args.call_info.name_tag.clone());

                return Ok(result?.to_output_stream());
            }
        }

        let scope = args.call_info.scope.clone();

        let input = args.input;
        let result = external::run_external_command(
            command,
            &mut external_context,
            input,
            &scope,
            external_redirection,
        )
        .await;

        Ok(result?.to_output_stream())
    }
}

#[allow(unused_variables)]
async fn maybe_autocd_dir<'a>(cmd: &ExternalCommand, ctx: &mut Context) -> Option<String> {
    // We will "auto cd" if
    //   - the command name ends in a path separator, or
    //   - it's not a command on the path and no arguments were given.
    let name = &cmd.name;
    let path_name = if name.ends_with(std::path::MAIN_SEPARATOR)
        || (cmd.args.is_empty()
            && PathBuf::from(name).is_dir()
            && dunce::canonicalize(name).is_ok()
            && !crate::commands::classified::external::did_find_command(&name))
    {
        Some(name)
    } else {
        None
    };

    path_name.map(|name| {
        #[cfg(windows)]
        {
            if name.ends_with(':') {
                // This looks like a drive shortcut. We need to a) switch drives and b) go back to the previous directory we were viewing on that drive
                // But first, we need to save where we are now
                let current_path = ctx.shell_manager.path();

                let split_path: Vec<_> = current_path.split(':').collect();
                if split_path.len() > 1 {
                    ctx.windows_drives_previous_cwd
                        .lock()
                        .insert(split_path[0].to_string(), current_path);
                }

                let name = name.to_uppercase();
                let new_drive: Vec<_> = name.split(':').collect();

                if let Some(val) = ctx.windows_drives_previous_cwd.lock().get(new_drive[0]) {
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
