use crate::commands::command::CallInfo;
use crate::prelude::*;
use crate::shell::shell::Shell;
use rustyline::completion::{self, Completer};
use rustyline::error::ReadlineError;
use rustyline::hint::Hinter;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ValueShell {
    crate path: String,
    crate value: Tagged<Value>,
}

impl ValueShell {
    pub fn new(value: Tagged<Value>) -> ValueShell {
        ValueShell {
            path: "/".to_string(),
            value,
        }
    }
    fn members(&self) -> VecDeque<Tagged<Value>> {
        let mut shell_entries = VecDeque::new();
        let full_path = PathBuf::from(&self.path);
        let mut viewed = self.value.clone();
        let sep_string = std::path::MAIN_SEPARATOR.to_string();
        let sep = OsStr::new(&sep_string);
        for p in full_path.iter() {
            match p {
                x if x == sep => {}
                step => match viewed.get_data_by_key(step.to_str().unwrap()) {
                    Some(v) => {
                        viewed = v.clone();
                    }
                    _ => {}
                },
            }
        }
        match viewed {
            Tagged {
                item: Value::List(l),
                ..
            } => {
                for item in l {
                    shell_entries.push_back(item.clone());
                }
            }
            x => {
                shell_entries.push_back(x.clone());
            }
        }

        shell_entries
    }
}

impl Shell for ValueShell {
    fn name(&self) -> String {
        "value".to_string()
    }

    fn ls(&self, _call_info: CallInfo, _input: InputStream) -> Result<OutputStream, ShellError> {
        Ok(self
            .members()
            .map(|x| ReturnSuccess::value(x))
            .to_output_stream())
    }

    fn cd(&self, call_info: CallInfo, _input: InputStream) -> Result<OutputStream, ShellError> {
        let path = match call_info.args.nth(0) {
            None => "/".to_string(),
            Some(v) => {
                let target = v.as_string()?;

                let mut cwd = PathBuf::from(&self.path);
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
                cwd.to_string_lossy().to_string()
            }
        };

        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::change_cwd(path));
        Ok(stream.into())
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn set_path(&mut self, path: String) {
        let _ = std::env::set_current_dir(&path);
        self.path = path.clone();
    }
}

impl Completer for ValueShell {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<completion::Pair>), ReadlineError> {
        let mut completions = vec![];

        let mut possible_completion = vec![];
        let members = self.members();
        for member in members {
            match member {
                Tagged { item, .. } => {
                    for desc in item.data_descriptors() {
                        possible_completion.push(desc);
                    }
                }
            }
        }

        let line_chars: Vec<_> = line.chars().collect();
        let mut replace_pos = pos;
        while replace_pos > 0 {
            if line_chars[replace_pos - 1] == ' ' {
                break;
            }
            replace_pos -= 1;
        }

        for command in possible_completion.iter() {
            let mut pos = replace_pos;
            let mut matched = true;
            if pos < line_chars.len() {
                for chr in command.chars() {
                    if line_chars[pos] != chr {
                        matched = false;
                        break;
                    }
                    pos += 1;
                    if pos == line_chars.len() {
                        break;
                    }
                }
            }

            if matched {
                completions.push(completion::Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }
        Ok((replace_pos, completions))
    }
}

impl Hinter for ValueShell {
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        None
    }
}
