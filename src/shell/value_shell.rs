use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::context::SourceMap;
use crate::prelude::*;
use crate::shell::shell::Shell;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ValueShell {
    pub(crate) path: String,
    pub(crate) value: Tagged<Value>,
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
    fn name(&self, source_map: &SourceMap) -> String {
        let origin_name = self.value.origin_name(source_map);
        format!(
            "{}",
            match origin_name {
                Some(x) => format!("{{{}}}", x),
                None => format!("<{}>", self.value.item.type_name(),),
            }
        )
    }

    fn homedir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn ls(&self, _args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        Ok(self
            .members()
            .map(|x| ReturnSuccess::value(x))
            .to_output_stream())
    }

    fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let path = match args.nth(0) {
            None => "/".to_string(),
            Some(v) => {
                let target = v.as_path()?;

                let mut cwd = PathBuf::from(&self.path);

                if target == PathBuf::from("..") {
                    cwd.pop();
                } else {
                    match target.to_str() {
                        Some(target) => match target.chars().nth(0) {
                            Some(x) if x == '/' => cwd = PathBuf::from(target),
                            _ => cwd.push(target),
                        },
                        None => cwd.push(target),
                    }
                }
                cwd.to_string_lossy().to_string()
            }
        };

        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::change_cwd(path));
        Ok(stream.into())
    }

    fn cp(&self, _args: CopyArgs, name: Span, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "cp not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn mv(&self, _args: MoveArgs, name: Span, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "mv not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn mkdir(&self, _args: MkdirArgs, name: Span, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "mkdir not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn rm(&self, _args: RemoveArgs, name: Span, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "rm not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn set_path(&mut self, path: String) {
        let _ = std::env::set_current_dir(&path);
        self.path = path.clone();
    }

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
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
                completions.push(rustyline::completion::Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }
        Ok((replace_pos, completions))
    }

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        None
    }
}
