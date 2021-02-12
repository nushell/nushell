use std::path::PathBuf;

use crate::prelude::*;
use nu_engine::filesystem::path::canonicalize;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct Arguments {
    #[serde(rename = "load")]
    pub load_path: Option<Tagged<PathBuf>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "nu plugin"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu plugin").named(
            "load",
            SyntaxShape::FilePath,
            "a path to load the plugins from",
            Some('l'),
        )
    }

    fn usage(&self) -> &str {
        "Nu Plugin"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Load all plugins in the current directory",
            example: "nu plugin --load .",
            result: None,
        }]
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let scope = args.scope.clone();
        let shell_manager = args.shell_manager.clone();
        let (Arguments { load_path }, _) = args.process().await?;

        if let Some(Tagged {
            item: load_path,
            tag,
        }) = load_path
        {
            let path = canonicalize(shell_manager.path(), load_path).map_err(|_| {
                ShellError::labeled_error(
                    "Cannot load plugins from directory",
                    "directory not found",
                    &tag,
                )
            })?;

            if !path.is_dir() {
                return Err(ShellError::labeled_error(
                    "Cannot load plugins from directory",
                    "is not a directory",
                    &tag,
                ));
            }

            #[cfg(unix)]
            {
                let has_exec = path
                    .metadata()
                    .map(|m| umask::Mode::from(m.permissions().mode()).has(umask::USER_READ))
                    .map_err(|e| {
                        ShellError::labeled_error(
                            "Cannot load plugins from directory",
                            format!("cannot stat ({})", e),
                            &tag,
                        )
                    })?;

                if !has_exec {
                    return Err(ShellError::labeled_error(
                        "Cannot load plugins from directory",
                        "permission denied",
                        &tag,
                    ));
                }
            }

            return Ok(vec![ReturnSuccess::action(CommandAction::AddPlugins(
                path.to_string_lossy().to_string(),
            ))]
            .into());
        }

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_help(&SubCommand, &scope)).into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
