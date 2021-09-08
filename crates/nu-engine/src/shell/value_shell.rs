use crate::maybe_text_codec::StringOrBinary;
use crate::shell::shell_args::{CdArgs, CopyArgs, LsArgs, MkdirArgs, MvArgs, RemoveArgs};
use crate::shell::Shell;
use crate::CommandArgs;
use encoding_rs::Encoding;
use nu_errors::ShellError;
use nu_protocol::ValueStructure;
use nu_protocol::{ReturnSuccess, ShellTypeName, UntaggedValue, Value};
use nu_source::SpannedItem;
use nu_source::{Span, Tag, Tagged};
use nu_stream::{ActionStream, OutputStream};
use nu_value_ext::ValueExt;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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

    pub fn find(&self, path: &Path) -> Option<&Self> {
        let mut value_system = ValueStructure::new();

        if value_system.walk_decorate(&self.value).is_ok() {
            value_system.exists(path).then(|| self)
        } else {
            None
        }
    }

    pub fn members_under(&self, path: &Path) -> VecDeque<Value> {
        let mut shell_entries = VecDeque::new();
        let full_path = path.to_path_buf();
        let mut viewed = self.value.clone();
        let sep_string = std::path::MAIN_SEPARATOR.to_string();
        let sep = OsStr::new(&sep_string);
        for p in &full_path {
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
    ) -> Result<ActionStream, ShellError> {
        let mut full_path = PathBuf::from(self.path());

        if let Some(value) = &path {
            full_path.push(&value.item);
        }

        if self.find(&full_path).is_none() {
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

    fn cd(&self, args: CdArgs, name: Tag) -> Result<ActionStream, ShellError> {
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

        Ok(ActionStream::one(ReturnSuccess::change_cwd(path)))
    }

    fn cp(&self, _args: CopyArgs, name: Tag, _path: &str) -> Result<ActionStream, ShellError> {
        Err(ShellError::labeled_error(
            "cp not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn mv(&self, _args: MvArgs, name: Tag, _path: &str) -> Result<ActionStream, ShellError> {
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

    fn rm(&self, _args: RemoveArgs, name: Tag, _path: &str) -> Result<ActionStream, ShellError> {
        Err(ShellError::labeled_error(
            "rm not currently supported on values",
            "not currently supported",
            name,
        ))
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn pwd(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(ActionStream::one(
            UntaggedValue::string(self.path()).into_value(&args.call_info.name_tag),
        ))
    }

    fn set_path(&mut self, path: String) {
        self.last_path = self.path.clone();
        self.path = path;
    }

    fn open(
        &self,
        _path: &Path,
        _name: Span,
        _with_encoding: Option<&'static Encoding>,
    ) -> Result<
        Box<dyn Iterator<Item = Result<StringOrBinary, ShellError>> + Send + Sync>,
        ShellError,
    > {
        Err(ShellError::unimplemented(
            "open on help shell is not supported",
        ))
    }

    fn save(
        &mut self,
        _path: &Path,
        _contents: &[u8],
        _name: Span,
        _append: bool,
    ) -> Result<OutputStream, ShellError> {
        Err(ShellError::unimplemented(
            "save on help shell is not supported",
        ))
    }

    fn is_interactive(&self) -> bool {
        //Value shell is always interactive
        true
    }
}
