use crate::errors::ShellError;
use crate::prelude::*;
use std::env;

pub fn cd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let cwd = args.env.lock().unwrap().cwd().to_path_buf();
    let path = match args.positional.first() {
        None => match dirs::home_dir() {
            Some(o) => o,
            _ => return Err(ShellError::string("Can not change to home directory")),
        },
        Some(v) => {
            let target = v.as_string()?.clone();
            match dunce::canonicalize(cwd.join(&target).as_path()) {
                Ok(p) => p,
                Err(_) => {
                    return Err(ShellError::labeled_error(
                        "Can not change to directory",
                        "directory not found",
                        args.positional[0].span.clone(),
                    ));
                }
            }
        }
    };

    let mut stream = VecDeque::new();
    match env::set_current_dir(&path) {
        Ok(_) => {}
        Err(_) => {
            if args.positional.len() > 0 {
                return Err(ShellError::labeled_error(
                    "Can not change to directory",
                    "directory not found",
                    args.positional[0].span.clone(),
                ));
            } else {
                return Err(ShellError::string("Can not change to directory"));
            }
        }
    }
    stream.push_back(ReturnValue::change_cwd(path));
    Ok(stream.boxed())
}
