use crate::errors::ShellError;
use crate::prelude::*;
use std::env;
use std::path::PathBuf;

pub fn cd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();
    let latest = env.back().unwrap();

    match latest.obj {
        Value::Filesystem => {
            let cwd = latest.path().to_path_buf();
            let path = match args.positional.first() {
                None => match dirs::home_dir() {
                    Some(o) => o,
                    _ => {
                        return Err(ShellError::maybe_labeled_error(
                            "Can not change to home directory",
                            "can not go to home",
                            args.name_span,
                        ))
                    }
                },
                Some(v) => {
                    let target = v.as_string()?.clone();
                    match dunce::canonicalize(cwd.join(&target).as_path()) {
                        Ok(p) => p,
                        Err(_) => {
                            return Err(ShellError::maybe_labeled_error(
                                "Can not change to directory",
                                "directory not found",
                                Some(args.positional[0].span.clone()),
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
                        return Err(ShellError::maybe_labeled_error(
                            "Can not change to directory",
                            "directory not found",
                            Some(args.positional[0].span.clone()),
                        ));
                    } else {
                        return Err(ShellError::string("Can not change to directory"));
                    }
                }
            }
            stream.push_back(ReturnValue::change_cwd(path));
            Ok(stream.boxed())
        }
        _ => {
            let mut stream = VecDeque::new();
            match args.positional.first() {
                None => {
                    stream.push_back(ReturnValue::change_cwd(PathBuf::from("/")));
                }
                Some(v) => {
                    let mut cwd = latest.path().to_path_buf();
                    let target = v.as_string()?.clone();
                    match target {
                        x if x == ".." => {
                            cwd.pop();
                        }
                        _ => match target.chars().nth(0) {
                            Some(x) if x == '/' => cwd = PathBuf::from(target),
                            _ => {
                                cwd.push(target);
                            }
                        },
                    }
                    stream.push_back(ReturnValue::change_cwd(cwd));
                }
            };
            Ok(stream.boxed())
        }
    }
}
