use crate::commands::cd::CdArgs;
use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::prelude::*;
use crate::shell::shell::Shell;
use crate::utils::ValueStructure;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use nu_source::Tagged;

#[derive(Clone)]
pub struct ValueShell {
    pub(crate) path: String,
    pub(crate) last_path: String,
    pub(crate) value: Value,
}

impl std::fmt::Debug for ValueShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValueShell @ {}", self.path)
    }
}

impl ValueShell {
    pub fn new(value: Value) -> ValueShell {
        ValueShell {
            path: "/".to_string(),
            last_path: "/".to_string(),
            value,
        }
    }

    fn members_under(&self, path: &Path) -> VecDeque<Value> {
        let mut shell_entries = VecDeque::new();
        let full_path = path.to_path_buf();
        let mut viewed = self.value.clone();
        let sep_string = std::path::MAIN_SEPARATOR.to_string();
        let sep = OsStr::new(&sep_string);
        for p in full_path.iter() {
            match p {
                x if x == sep => {}
                step => {
                    let name: &str = &step.to_string_lossy().to_string();
                    let value = viewed.get_data_by_key(name.spanned_unknown());
                    if let Some(v) = value {
                        viewed = v.clone();
                    }
                }
            }
        }
        match viewed {
            Value {
                value: UntaggedValue::Table(l),
                ..
            } => {
                for item in l {
                    shell_entries.push_back(item.clone());
                }
            }
            x => {
                shell_entries.push_back(x);
            }
        }

        shell_entries
    }

    fn members(&self) -> VecDeque<Value> {
        self.members_under(Path::new("."))
    }
}

impl Shell for ValueShell {
    fn name(&self) -> String {
        let anchor_name = self.value.anchor_name();

        match anchor_name {
            Some(x) => format!("{{{}}}", x),
            None => format!("<{}>", self.value.type_name()),
        }
    }

    fn homedir(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/"))
    }

    fn ls(
        &self,
        LsArgs { path, .. }: LsArgs,
        name_tag: Tag,
        _ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError> {
        let mut full_path = PathBuf::from(self.path());

        if let Some(value) = &path {
            full_path.push(value.as_ref());
        }

        let mut value_system = ValueStructure::new();
        value_system.walk_decorate(&self.value)?;

        if !value_system.exists(&full_path) {
            if let Some(target) = &path {
                return Err(ShellError::labeled_error(
                    "Can not list entries inside",
                    "No such path exists",
                    target.tag(),
                ));
            }

            return Err(ShellError::labeled_error(
                "Can not list entries inside",
                "No such path exists",
                name_tag,
            ));
        }

        let output = self
            .members_under(full_path.as_path())
            .into_iter()
            .map(ReturnSuccess::value)
            .collect::<VecDeque<_>>();
        Ok(output.into())
    }

    fn cd(&self, args: CdArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let destination = args.path;

        let path = match destination {
            None => "/".to_string(),
            Some(ref v) => {
                let Tagged { item: target, .. } = v;
                let mut cwd = PathBuf::from(&self.path);

                if target == &PathBuf::from("..") {
                    cwd.pop();
                } else if target == &PathBuf::from("-") {
                    cwd = PathBuf::from(&self.last_path);
                } else {
                    match target.to_str() {
                        Some(target) => match target.chars().next() {
                            Some(x) if x == '/' => cwd = PathBuf::from(target),
                            _ => cwd.push(target),
                        },
                        None => cwd.push(target),
                    }
                }
                cwd.to_string_lossy().to_string()
            }
        };

        let mut value_system = ValueStructure::new();
        value_system.walk_decorate(&self.value)?;

        if !value_system.exists(&PathBuf::from(&path)) {
            if let Some(destination) = destination {
                return Err(ShellError::labeled_error(
                    "Can not change to path inside",
                    "No such path exists",
                    destination.tag(),
                ));
            }

            return Err(ShellError::labeled_error(
                "Can not change to path inside",
                "No such path exists",
                &name,
            ));
        }

        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::change_cwd(path));
        Ok(stream.into())
    }

    fn cp(&self, _args: CopyArgs, name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "cp not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn mv(&self, _args: MoveArgs, name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "mv not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn mkdir(&self, _args: MkdirArgs, name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "mkdir not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn rm(&self, _args: RemoveArgs, name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Err(ShellError::labeled_error(
            "rm not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::value(
            UntaggedValue::string(self.path()).into_value(&args.call_info.name_tag),
        ));
        Ok(stream.into())
    }

    fn set_path(&mut self, path: String) {
        self.last_path = self.path.clone();
        self.path = path;
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
            let Value { value, .. } = member;
            for desc in value.data_descriptors() {
                possible_completion.push(desc);
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
