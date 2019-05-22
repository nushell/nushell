use crate::errors::ShellError;
use crate::prelude::*;
use std::env;

pub fn cd(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let target = match args.args.first() {
        // TODO: This needs better infra
        None => return Err(ShellError::string(format!("cd must take one arg"))),
        Some(v) => v.as_string()?.clone(),
    };

    let cwd = args.env.cwd().to_path_buf();

    let mut stream = VecDeque::new();
    let path = dunce::canonicalize(cwd.join(&target).as_path())?;
    let _ = env::set_current_dir(&path);
    stream.push_back(ReturnValue::change_cwd(path));
    Ok(stream)
}
