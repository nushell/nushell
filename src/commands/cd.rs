use crate::errors::ShellError;
use crate::prelude::*;
use std::env;

pub fn cd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();
    let cwd = env.path().to_path_buf();

    let path = match args.nth(0) {
        None => match dirs::home_dir() {
            Some(o) => o,
            _ => {
                return Err(ShellError::labeled_error(
                    "Can not change to home directory",
                    "can not go to home",
                    args.call_info.name_span,
                ))
            }
        },
        Some(v) => {
            let target = v.as_string()?;
            match dunce::canonicalize(cwd.join(target).as_path()) {
                Ok(p) => p,
                Err(_) => {
                    return Err(ShellError::labeled_error(
                        "Can not change to directory",
                        "directory not found",
                        v.span().clone(),
                    ));
                }
            }
        }
    };

    let mut stream = VecDeque::new();
    match env::set_current_dir(&path) {
        Ok(_) => {}
        Err(_) => {
            if args.len() > 0 {
                return Err(ShellError::labeled_error(
                    "Can not change to directory",
                    "directory not found",
                    args.nth(0).unwrap().span().clone(),
                ));
            } else {
                return Err(ShellError::string("Can not change to directory"));
            }
        }
    }
    stream.push_back(ReturnSuccess::change_cwd(path));
    Ok(stream.into())
}
