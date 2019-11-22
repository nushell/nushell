use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::data::{command_dict, TaggedDictBuilder};
use crate::prelude::*;
use crate::shell::shell::Shell;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct HelpShell {
    pub(crate) path: String,
    pub(crate) value: Tagged<Value>,
}

impl HelpShell {
    pub fn index(registry: &CommandRegistry) -> Result<HelpShell, std::io::Error> {
        let mut cmds = TaggedDictBuilder::new(Tag::unknown());
        let mut specs = Vec::new();

        for cmd in registry.names() {
            let mut spec = TaggedDictBuilder::new(Tag::unknown());
            let value = command_dict(registry.get_command(&cmd).unwrap(), Tag::unknown());

            spec.insert("name", cmd);
            spec.insert(
                "description",
                value
                    .get_data_by_key("usage".spanned_unknown())
                    .unwrap()
                    .as_string()
                    .unwrap(),
            );
            spec.insert_tagged("details", value);

            specs.push(spec.into_tagged_value());
        }

        cmds.insert("help", Value::Table(specs));

        Ok(HelpShell {
            path: "/help".to_string(),
            value: cmds.into_tagged_value(),
        })
    }

    pub fn for_command(
        cmd: Tagged<Value>,
        registry: &CommandRegistry,
    ) -> Result<HelpShell, std::io::Error> {
        let mut sh = HelpShell::index(&registry)?;

        if let Tagged {
            item: Value::Primitive(Primitive::String(name)),
            ..
        } = cmd
        {
            sh.set_path(format!("/help/{:}/details", name));
        }

        Ok(sh)
    }

    fn commands(&self) -> VecDeque<Tagged<Value>> {
        let mut cmds = VecDeque::new();
        let full_path = PathBuf::from(&self.path);

        let mut viewed = self.value.clone();
        let sep_string = std::path::MAIN_SEPARATOR.to_string();
        let sep = OsStr::new(&sep_string);

        for p in full_path.iter() {
            match p {
                x if x == sep => {}
                step => match viewed.get_data_by_key(step.to_str().unwrap().spanned_unknown()) {
                    Some(v) => {
                        viewed = v.clone();
                    }
                    _ => {}
                },
            }
        }
        match viewed {
            Tagged {
                item: Value::Table(l),
                ..
            } => {
                for item in l {
                    cmds.push_back(item.clone());
                }
            }
            x => {
                cmds.push_back(x.clone());
            }
        }

        cmds
    }
}

impl Shell for HelpShell {
    fn name(&self) -> String {
        let anchor_name = self.value.anchor_name();
        format!(
            "{}",
            match anchor_name {
                Some(x) => format!("{{{}}}", x),
                None => format!("<{}>", self.value.item.type_name(),),
            }
        )
    }

    fn homedir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn pwd(&self, _: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn set_path(&mut self, path: String) {
        let _ = std::env::set_current_dir(&path);
        self.path = path.clone();
    }

    fn ls(
        &self,
        _pattern: Option<Tagged<PathBuf>>,
        _context: &RunnableContext,
        _full: bool,
    ) -> Result<OutputStream, ShellError> {
        Ok(self
            .commands()
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

    fn cp(&self, _args: CopyArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn mv(&self, _args: MoveArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn mkdir(&self, _args: MkdirArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn rm(&self, _args: RemoveArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        let mut completions = vec![];

        let mut possible_completion = vec![];
        let commands = self.commands();
        for cmd in commands {
            match cmd {
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
