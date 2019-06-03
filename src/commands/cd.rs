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
            dunce::canonicalize(cwd.join(&target).as_path())?
        }
    };

    let mut stream = VecDeque::new();
    let _ = env::set_current_dir(&path);
    stream.push_back(ReturnValue::change_cwd(path));
    Ok(stream.boxed())
}
