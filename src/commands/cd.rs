use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::prelude::*;
use std::env;
use std::path::PathBuf;

pub struct Cd;

#[derive(Deserialize)]
pub struct CdArgs {
    target: Option<Spanned<PathBuf>>,
}

impl StaticCommand for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn signature(&self) -> Signature {
        Signature::build("cd")
            .optional("target", SyntaxType::Path)
            .filter()
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, cd)?.run()
        // cd(args, registry)
    }
}

pub fn cd(CdArgs { target }: CdArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let cwd = context.cwd().to_path_buf();

    let path = match &target {
        None => match dirs::home_dir() {
            Some(o) => o,
            _ => {
                return Err(ShellError::maybe_labeled_error(
                    "Can not change to home directory",
                    "can not go to home",
                    context.name,
                ))
            }
        },
        Some(v) => {
            // let target = v.item.as_string()?;
            match dunce::canonicalize(cwd.join(&v.item()).as_path()) {
                Ok(p) => p,
                Err(_) => {
                    return Err(ShellError::labeled_error(
                        "Can not change to directory",
                        "directory not found",
                        v.span.clone(),
                    ));
                }
            }
        }
    };

    let mut stream = VecDeque::new();
    match env::set_current_dir(&path) {
        Ok(_) => {}
        Err(_) => {
            if let Some(path) = target {
                return Err(ShellError::labeled_error(
                    "Can not change to directory",
                    "directory not found",
                    path.span,
                ));
            } else {
                return Err(ShellError::string("Can not change to directory"));
            }
        }
    }
    stream.push_back(ReturnSuccess::change_cwd(path));
    Ok(stream.into())
}
