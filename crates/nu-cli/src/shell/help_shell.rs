use crate::commands::cd::CdArgs;
use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::move_::mv::Arguments as MvArgs;
use crate::commands::rm::RemoveArgs;
use crate::completion;
use crate::data::command_dict;
use crate::prelude::*;
use crate::shell::shell::Shell;

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::commands::classified::maybe_text_codec::StringOrBinary;
use encoding_rs::Encoding;
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, ShellTypeName, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;

#[derive(Clone, Debug)]
pub struct HelpShell {
    pub(crate) path: String,
    pub(crate) value: Value,
}

impl HelpShell {
    pub fn index(registry: &CommandRegistry) -> Result<HelpShell, ShellError> {
        let mut cmds = TaggedDictBuilder::new(Tag::unknown());
        let mut specs = Vec::new();

        for cmd in registry.names() {
            if let Some(cmd_value) = registry.get_command(&cmd) {
                let mut spec = TaggedDictBuilder::new(Tag::unknown());
                let value = command_dict(cmd_value, Tag::unknown());

                spec.insert_untagged("name", cmd);
                spec.insert_untagged(
                    "description",
                    value
                        .get_data_by_key("usage".spanned_unknown())
                        .ok_or_else(|| {
                            ShellError::untagged_runtime_error(
                                "Internal error: expected to find usage",
                            )
                        })?
                        .as_string()?,
                );
                spec.insert_value("details", value);

                specs.push(spec.into_value());
            } else {
            }
        }

        cmds.insert_untagged("help", UntaggedValue::Table(specs));

        Ok(HelpShell {
            path: "/help".to_string(),
            value: cmds.into_value(),
        })
    }

    pub fn for_command(cmd: Value, registry: &CommandRegistry) -> Result<HelpShell, ShellError> {
        let mut sh = HelpShell::index(&registry)?;

        if let Value {
            value: UntaggedValue::Primitive(Primitive::String(name)),
            ..
        } = cmd
        {
            sh.set_path(format!("/help/{:}/details", name));
        }

        Ok(sh)
    }

    fn commands(&self) -> VecDeque<Value> {
        let mut cmds = VecDeque::new();
        let full_path = PathBuf::from(&self.path);

        let mut viewed = self.value.clone();
        let sep_string = std::path::MAIN_SEPARATOR.to_string();
        let sep = OsStr::new(&sep_string);

        for p in full_path.iter() {
            match p {
                x if x == sep => {}
                step => {
                    let step: &str = &step.to_string_lossy().to_string();
                    let value = viewed.get_data_by_key(step.spanned_unknown());
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
                    cmds.push_back(item.clone());
                }
            }
            x => {
                cmds.push_back(x);
            }
        }

        cmds
    }
}

impl Shell for HelpShell {
    fn name(&self) -> String {
        let anchor_name = self.value.anchor_name();

        match anchor_name {
            Some(x) => format!("{{{}}}", x),
            None => format!("<{}>", self.value.type_name()),
        }
    }

    fn homedir(&self) -> Option<PathBuf> {
        #[cfg(feature = "dirs")]
        {
            dirs::home_dir()
        }

        #[cfg(not(feature = "dirs"))]
        {
            None
        }
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn pwd(&self, _: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn set_path(&mut self, path: String) {
        let _ = std::env::set_current_dir(&path);
        self.path = path;
    }

    fn ls(
        &self,
        _args: LsArgs,
        _name: Tag,
        _ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError> {
        let output = self
            .commands()
            .into_iter()
            .map(ReturnSuccess::value)
            .collect::<VecDeque<_>>();
        Ok(output.into())
    }

    fn cd(&self, args: CdArgs, _name: Tag) -> Result<OutputStream, ShellError> {
        let path = match args.path {
            None => "/".to_string(),
            Some(v) => {
                let Tagged { item: target, .. } = v;
                let mut cwd = PathBuf::from(&self.path);

                if target == PathBuf::from("..") {
                    cwd.pop();
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

        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::change_cwd(path));
        Ok(stream.into())
    }

    fn cp(&self, _args: CopyArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn mv(&self, _args: MvArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn mkdir(&self, _args: MkdirArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn rm(&self, _args: RemoveArgs, _name: Tag, _path: &str) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::empty())
    }

    fn open(
        &self,
        _path: &PathBuf,
        _name: Span,
        _with_encoding: Option<&'static Encoding>,
    ) -> Result<BoxStream<'static, Result<StringOrBinary, ShellError>>, ShellError> {
        Err(ShellError::unimplemented(
            "open on help shell is not supported",
        ))
    }

    fn save(
        &mut self,
        _path: &PathBuf,
        _contents: &[u8],
        _name: Span,
    ) -> Result<OutputStream, ShellError> {
        Err(ShellError::unimplemented(
            "save on help shell is not supported",
        ))
    }
}

impl completion::Completer for HelpShell {
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &completion::Context<'_>,
    ) -> Result<(usize, Vec<completion::Suggestion>), ShellError> {
        let mut possible_completion = vec![];
        let commands = self.commands();
        for cmd in commands {
            let Value { value, .. } = cmd;
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

        let mut completions = vec![];
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
                completions.push(completion::Suggestion {
                    display: command.to_string(),
                    replacement: command.to_string(),
                });
            }
        }
        Ok((replace_pos, completions))
    }

    fn hint(&self, _line: &str, _pos: usize, _ctx: &completion::Context<'_>) -> Option<String> {
        None
    }
}
