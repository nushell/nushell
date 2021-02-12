use crate::command_args::EvaluatedWholeStreamCommandArgs;
use crate::filesystem::dir_info::{DirBuilder, DirInfo};
use crate::filesystem::path::canonicalize;
use crate::filesystem::utils::FileStructure;
use crate::maybe_text_codec::{MaybeTextCodec, StringOrBinary};
use crate::shell::shell_args::{CdArgs, CopyArgs, LsArgs, MkdirArgs, MvArgs, RemoveArgs};
use crate::shell::Shell;
use encoding_rs::Encoding;
use futures::stream::BoxStream;
use futures::StreamExt;
use futures_codec::FramedRead;
use futures_util::TryStreamExt;
use nu_protocol::{TaggedDictBuilder, Value};
use nu_source::{Span, Tag};
use nu_stream::{Interruptible, OutputStream, ToOutputStream};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue};
use nu_source::Tagged;

pub struct FilesystemShell {
    pub(crate) path: String,
    pub(crate) last_path: String,
}

impl std::fmt::Debug for FilesystemShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilesystemShell @ {}", self.path)
    }
}

impl Clone for FilesystemShell {
    fn clone(&self) -> Self {
        FilesystemShell {
            path: self.path.clone(),
            last_path: self.path.clone(),
        }
    }
}

impl FilesystemShell {
    pub fn basic() -> Result<FilesystemShell, Error> {
        let path = match std::env::current_dir() {
            Ok(path) => path,
            Err(_) => PathBuf::from("/"),
        };

        Ok(FilesystemShell {
            path: path.to_string_lossy().to_string(),
            last_path: path.to_string_lossy().to_string(),
        })
    }

    pub fn with_location(path: String) -> Result<FilesystemShell, std::io::Error> {
        let path = canonicalize(std::env::current_dir()?, &path)?;
        let path = path.display().to_string();
        let last_path = path.clone();

        Ok(FilesystemShell { path, last_path })
    }
}

pub fn homedir_if_possible() -> Option<PathBuf> {
    #[cfg(feature = "dirs")]
    {
        dirs_next::home_dir()
    }

    #[cfg(not(feature = "dirs"))]
    {
        None
    }
}

impl Shell for FilesystemShell {
    fn name(&self) -> String {
        "filesystem".to_string()
    }

    fn homedir(&self) -> Option<PathBuf> {
        homedir_if_possible()
    }

    fn ls(
        &self,
        LsArgs {
            path,
            all,
            long,
            short_names,
            du,
        }: LsArgs,
        name_tag: Tag,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError> {
        let ctrl_c_copy = ctrl_c.clone();
        let (path, p_tag) = match path {
            Some(p) => {
                let p_tag = p.tag;
                let mut p = p.item;
                if p.is_dir() {
                    if is_empty_dir(&p) {
                        return Ok(OutputStream::empty());
                    }
                    p.push("*");
                }
                (p, p_tag)
            }
            None => {
                if is_empty_dir(&self.path()) {
                    return Ok(OutputStream::empty());
                } else {
                    (PathBuf::from("./*"), name_tag.clone())
                }
            }
        };

        let hidden_dir_specified = is_hidden_dir(&path);

        let mut paths = glob::glob(&path.to_string_lossy())
            .map_err(|e| ShellError::labeled_error(e.to_string(), "invalid pattern", &p_tag))?
            .peekable();

        if paths.peek().is_none() {
            return Err(ShellError::labeled_error(
                "No matches found",
                "no matches found",
                &p_tag,
            ));
        }

        let mut hidden_dirs = vec![];

        // Generated stream: impl Stream<Item = Result<ReturnSuccess, ShellError>

        Ok(futures::stream::iter(paths.filter_map(move |path| {
            let path = match path.map_err(|e| ShellError::from(e.into_error())) {
                Ok(path) => path,
                Err(err) => return Some(Err(err)),
            };

            if path_contains_hidden_folder(&path, &hidden_dirs) {
                return None;
            }

            if !all && !hidden_dir_specified && is_hidden_dir(&path) {
                if path.is_dir() {
                    hidden_dirs.push(path);
                }
                return None;
            }

            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => Some(metadata),
                Err(e) => {
                    if e.kind() == ErrorKind::PermissionDenied || e.kind() == ErrorKind::Other {
                        None
                    } else {
                        return Some(Err(e.into()));
                    }
                }
            };

            let entry = dir_entry_dict(
                &path,
                metadata.as_ref(),
                name_tag.clone(),
                long,
                short_names,
                du,
                ctrl_c.clone(),
            )
            .map(ReturnSuccess::Value);

            Some(entry)
        }))
        .interruptible(ctrl_c_copy)
        .to_output_stream())
    }

    fn cd(&self, args: CdArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let path = match args.path {
            None => match homedir_if_possible() {
                Some(o) => o,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Cannot change to home directory",
                        "cannot go to home",
                        &name,
                    ))
                }
            },
            Some(v) => {
                let Tagged { item: target, tag } = v;
                if target == Path::new("-") {
                    PathBuf::from(&self.last_path)
                } else {
                    let path = canonicalize(self.path(), target).map_err(|_| {
                        ShellError::labeled_error(
                            "Cannot change to directory",
                            "directory not found",
                            &tag,
                        )
                    })?;

                    if !path.is_dir() {
                        return Err(ShellError::labeled_error(
                            "Cannot change to directory",
                            "is not a directory",
                            &tag,
                        ));
                    }

                    #[cfg(unix)]
                    {
                        let has_exec = path
                            .metadata()
                            .map(|m| {
                                umask::Mode::from(m.permissions().mode()).has(umask::USER_READ)
                            })
                            .map_err(|e| {
                                ShellError::labeled_error(
                                    "Cannot change to directory",
                                    format!("cannot stat ({})", e),
                                    &tag,
                                )
                            })?;

                        if !has_exec {
                            return Err(ShellError::labeled_error(
                                "Cannot change to directory",
                                "permission denied",
                                &tag,
                            ));
                        }
                    }

                    path
                }
            }
        };

        let mut stream = VecDeque::new();

        stream.push_back(ReturnSuccess::change_cwd(
            path.to_string_lossy().to_string(),
        ));

        Ok(stream.into())
    }

    fn cp(
        &self,
        CopyArgs {
            src,
            dst,
            recursive,
        }: CopyArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let name_tag = name;

        let path = Path::new(path);
        let source = path.join(&src.item);
        let destination = path.join(&dst.item);

        let sources: Vec<_> = match glob::glob(&source.to_string_lossy()) {
            Ok(files) => files.collect(),
            Err(e) => {
                return Err(ShellError::labeled_error(
                    e.to_string(),
                    "invalid pattern",
                    src.tag,
                ))
            }
        };

        if sources.is_empty() {
            return Err(ShellError::labeled_error(
                "No matches found",
                "no matches found",
                src.tag,
            ));
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::labeled_error(
                "Destination must be a directory when copying multiple files",
                "is not a directory",
                dst.tag,
            ));
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));

        if any_source_is_dir && !recursive.item {
            return Err(ShellError::labeled_error(
                "Directories must be copied using \"--recursive\"",
                "resolves to a directory (not copied)",
                src.tag,
            ));
        }

        for entry in sources {
            if let Ok(entry) = entry {
                let mut sources = FileStructure::new();
                sources.walk_decorate(&entry)?;

                if entry.is_file() {
                    let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
                        if destination.is_dir() {
                            let mut dest = canonicalize(&path, &dst.item)?;
                            if let Some(name) = entry.file_name() {
                                dest.push(name);
                            }
                            Ok((source_file, dest))
                        } else {
                            Ok((source_file, destination.clone()))
                        }
                    })?;

                    for (src, dst) in sources {
                        if src.is_file() {
                            std::fs::copy(src, dst).map_err(|e| {
                                ShellError::labeled_error(e.to_string(), e.to_string(), &name_tag)
                            })?;
                        }
                    }
                } else if entry.is_dir() {
                    let destination = if !destination.exists() {
                        destination.clone()
                    } else {
                        match entry.file_name() {
                            Some(name) => destination.join(name),
                            None => {
                                return Err(ShellError::labeled_error(
                                    "Copy aborted. Not a valid path",
                                    "not a valid path",
                                    dst.tag,
                                ))
                            }
                        }
                    };

                    std::fs::create_dir_all(&destination).map_err(|e| {
                        ShellError::labeled_error(e.to_string(), e.to_string(), &dst.tag)
                    })?;

                    let sources = sources.paths_applying_with(|(source_file, depth_level)| {
                        let mut dest = destination.clone();
                        let path = canonicalize(&path, &source_file)?;

                        let comps: Vec<_> = path
                            .components()
                            .map(|fragment| fragment.as_os_str())
                            .rev()
                            .take(1 + depth_level)
                            .collect();

                        for fragment in comps.into_iter().rev() {
                            dest.push(fragment);
                        }

                        Ok((PathBuf::from(&source_file), dest))
                    })?;

                    let dst_tag = &dst.tag;
                    for (src, dst) in sources {
                        if src.is_dir() && !dst.exists() {
                            std::fs::create_dir_all(&dst).map_err(|e| {
                                ShellError::labeled_error(e.to_string(), e.to_string(), dst_tag)
                            })?;
                        }

                        if src.is_file() {
                            std::fs::copy(&src, &dst).map_err(|e| {
                                ShellError::labeled_error(e.to_string(), e.to_string(), &name_tag)
                            })?;
                        }
                    }
                }
            }
        }

        Ok(OutputStream::empty())
    }

    fn mkdir(
        &self,
        MkdirArgs {
            rest: directories,
            show_created_paths,
        }: MkdirArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let path = Path::new(path);
        let mut stream = VecDeque::new();

        if directories.is_empty() {
            return Err(ShellError::labeled_error(
                "mkdir requires directory paths",
                "needs parameter",
                name,
            ));
        }

        for dir in directories.iter() {
            let create_at = path.join(&dir.item);

            let dir_res = std::fs::create_dir_all(&create_at);
            if let Err(reason) = dir_res {
                return Err(ShellError::labeled_error(
                    reason.to_string(),
                    reason.to_string(),
                    dir.tag(),
                ));
            }
            if show_created_paths {
                let val = format!("{:}", create_at.to_string_lossy()).into();
                stream.push_back(Ok(ReturnSuccess::Value(val)));
            }
        }

        Ok(stream.into())
    }

    fn mv(
        &self,
        MvArgs { src, dst }: MvArgs,
        _name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let path = Path::new(path);
        let source = path.join(&src.item);
        let destination = path.join(&dst.item);

        let mut sources =
            glob::glob(&source.to_string_lossy()).map_or_else(|_| Vec::new(), Iterator::collect);

        if sources.is_empty() {
            return Err(ShellError::labeled_error(
                "Invalid file or pattern",
                "invalid file or pattern",
                src.tag,
            ));
        }

        // We have two possibilities.
        //
        // First, the destination exists.
        //  - If a directory, move everything into that directory, otherwise
        //  - if only a single source, overwrite the file, otherwise
        //  - error.
        //
        // Second, the destination doesn't exist, so we can only rename a single source. Otherwise
        // it's an error.

        if (destination.exists() && !destination.is_dir() && sources.len() > 1)
            || (!destination.exists() && sources.len() > 1)
        {
            return Err(ShellError::labeled_error(
                "Can only move multiple sources if destination is a directory",
                "destination must be a directory when multiple sources",
                dst.tag,
            ));
        }

        let some_if_source_is_destination = sources
            .iter()
            .find(|f| matches!(f, Ok(f) if destination.starts_with(f)));
        if destination.exists() && destination.is_dir() && sources.len() == 1 {
            if let Some(Ok(filename)) = some_if_source_is_destination {
                return Err(ShellError::labeled_error(
                    format!(
                        "Not possible to move {:?} to itself",
                        filename.file_name().expect("Invalid file name")
                    ),
                    "cannot move to itself",
                    dst.tag,
                ));
            }
        }

        if let Some(Ok(_filename)) = some_if_source_is_destination {
            sources = sources
                .into_iter()
                .filter(|f| matches!(f, Ok(f) if !destination.starts_with(f)))
                .collect();
        }

        for entry in sources {
            if let Ok(entry) = entry {
                move_file(
                    TaggedPathBuf(&entry, &src.tag),
                    TaggedPathBuf(&destination, &dst.tag),
                )?
            }
        }

        Ok(OutputStream::empty())
    }

    fn rm(
        &self,
        RemoveArgs {
            rest: targets,
            recursive,
            trash: _trash,
            permanent: _permanent,
            force: _force,
        }: RemoveArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let name_tag = name;

        if targets.is_empty() {
            return Err(ShellError::labeled_error(
                "rm requires target paths",
                "needs parameter",
                name_tag,
            ));
        }

        let path = Path::new(path);
        let mut all_targets: HashMap<PathBuf, Tag> = HashMap::new();
        for target in targets {
            let all_dots = target
                .item
                .to_str()
                .map_or(false, |v| v.chars().all(|c| c == '.'));

            if all_dots {
                return Err(ShellError::labeled_error(
                    "Cannot remove any parent directory",
                    "cannot remove any parent directory",
                    target.tag,
                ));
            }

            let path = path.join(&target.item);
            match glob::glob(&path.to_string_lossy()) {
                Ok(files) => {
                    for file in files {
                        match file {
                            Ok(ref f) => {
                                all_targets
                                    .entry(f.clone())
                                    .or_insert_with(|| target.tag.clone());
                            }
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!("Could not remove {:}", path.to_string_lossy()),
                                    e.to_string(),
                                    &target.tag,
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(ShellError::labeled_error(
                        e.to_string(),
                        e.to_string(),
                        &name_tag,
                    ))
                }
            };
        }

        if all_targets.is_empty() && !_force.item {
            return Err(ShellError::labeled_error(
                "No valid paths",
                "no valid paths",
                name_tag,
            ));
        }

        Ok(
            futures::stream::iter(all_targets.into_iter().map(move |(f, tag)| {
                let is_empty = || match f.read_dir() {
                    Ok(mut p) => p.next().is_none(),
                    Err(_) => false,
                };

                if let Ok(metadata) = f.symlink_metadata() {
                    #[cfg(unix)]
                    let is_socket = metadata.file_type().is_socket();
                    #[cfg(not(unix))]
                    let is_socket = false;

                    if metadata.is_file()
                        || metadata.file_type().is_symlink()
                        || recursive.item
                        || is_socket
                        || is_empty()
                    {
                        let result;
                        #[cfg(feature = "trash-support")]
                        {
                            let rm_always_trash = config::config(Tag::unknown())?
                                .get("rm_always_trash")
                                .map(|val| val.is_true())
                                .unwrap_or(false);
                            result = if _trash.item || (rm_always_trash && !_permanent.item) {
                                trash::delete(&f).map_err(|e: trash::Error| {
                                    Error::new(ErrorKind::Other, format!("{:?}", e))
                                })
                            } else if metadata.is_file() {
                                std::fs::remove_file(&f)
                            } else {
                                std::fs::remove_dir_all(&f)
                            };
                        }
                        #[cfg(not(feature = "trash-support"))]
                        {
                            result = if metadata.is_file() || is_socket {
                                std::fs::remove_file(&f)
                            } else {
                                std::fs::remove_dir_all(&f)
                            };
                        }

                        if let Err(e) = result {
                            let msg =
                                format!("Could not delete because: {:}\nTry '--trash' flag", e);
                            Err(ShellError::labeled_error(msg, e.to_string(), tag))
                        } else {
                            let val = format!("deleted {:}", f.to_string_lossy()).into();
                            Ok(ReturnSuccess::Value(val))
                        }
                    } else {
                        let msg =
                            format!("Cannot remove {:}. try --recursive", f.to_string_lossy());
                        Err(ShellError::labeled_error(
                            msg,
                            "cannot remove non-empty directory",
                            tag,
                        ))
                    }
                } else {
                    let msg = format!("no such file or directory: {:}", f.to_string_lossy());
                    Err(ShellError::labeled_error(
                        msg,
                        "no such file or directory",
                        tag,
                    ))
                }
            }))
            .to_output_stream(),
        )
    }

    fn path(&self) -> String {
        self.path.clone()
    }

    fn pwd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let path = PathBuf::from(self.path());
        let p = match dunce::canonicalize(path.as_path()) {
            Ok(p) => p,
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "unable to show current directory",
                    "pwd command failed",
                    &args.call_info.name_tag,
                ));
            }
        };

        let mut stream = VecDeque::new();
        stream.push_back(ReturnSuccess::value(
            UntaggedValue::Primitive(Primitive::String(p.to_string_lossy().to_string()))
                .into_value(&args.call_info.name_tag),
        ));

        Ok(stream.into())
    }

    fn set_path(&mut self, path: String) {
        let pathbuf = PathBuf::from(&path);
        let path = match canonicalize(self.path(), pathbuf.as_path()) {
            Ok(path) => {
                let _ = std::env::set_current_dir(&path);
                std::env::set_var("PWD", &path);
                path
            }
            _ => {
                // TODO: handle the case where the path cannot be canonicalized
                pathbuf
            }
        };
        self.last_path = self.path.clone();
        self.path = path.to_string_lossy().to_string();
    }

    fn open(
        &self,
        path: &Path,
        name: Span,
        with_encoding: Option<&'static Encoding>,
    ) -> Result<BoxStream<'static, Result<StringOrBinary, ShellError>>, ShellError> {
        let metadata = std::fs::metadata(&path);

        let read_full = if let Ok(metadata) = metadata {
            // Arbitrarily capping the file at 32 megs, so we don't try to read large files in all at once
            metadata.is_file() && metadata.len() < (1024 * 1024 * 32)
        } else {
            false
        };

        if read_full {
            use futures_codec::Decoder;

            // We should, in theory, be able to read in the whole file as one chunk
            let buffer = std::fs::read(&path).map_err(|e| {
                ShellError::labeled_error(
                    format!("Error opening file: {:?}", e),
                    "Error opening file",
                    name,
                )
            })?;

            let mut bytes_mut = bytes::BytesMut::from(&buffer[..]);

            let mut codec = MaybeTextCodec::new(with_encoding);

            match codec.decode(&mut bytes_mut).map_err(|_| {
                ShellError::labeled_error("Error opening file", "error opening file", name)
            })? {
                Some(sb) => Ok(futures::stream::iter(vec![Ok(sb)].into_iter()).boxed()),
                None => Ok(futures::stream::iter(vec![].into_iter()).boxed()),
            }
        } else {
            // We don't know that this is a finite file, so treat it as a stream
            let f = std::fs::File::open(&path).map_err(|e| {
                ShellError::labeled_error(
                    format!("Error opening file: {:?}", e),
                    "Error opening file",
                    name,
                )
            })?;
            let async_reader = futures::io::AllowStdIo::new(f);
            let sob_stream = FramedRead::new(async_reader, MaybeTextCodec::new(with_encoding))
                .map_err(move |_| {
                    ShellError::labeled_error("Error opening file", "error opening file", name)
                })
                .into_stream();

            Ok(sob_stream.boxed())
        }
    }

    fn save(
        &mut self,
        full_path: &Path,
        save_data: &[u8],
        name: Span,
    ) -> Result<OutputStream, ShellError> {
        match std::fs::write(full_path, save_data) {
            Ok(_) => Ok(OutputStream::empty()),
            Err(e) => Err(ShellError::labeled_error(
                e.to_string(),
                "IO error while saving",
                name,
            )),
        }
    }
}

struct TaggedPathBuf<'a>(&'a PathBuf, &'a Tag);

fn move_file(from: TaggedPathBuf, to: TaggedPathBuf) -> Result<(), ShellError> {
    let TaggedPathBuf(from, from_tag) = from;
    let TaggedPathBuf(to, to_tag) = to;

    if to.exists() && from.is_dir() && to.is_file() {
        return Err(ShellError::labeled_error(
            "Cannot rename a directory to a file",
            "invalid destination",
            to_tag,
        ));
    }

    let destination_dir_exists = if to.is_dir() {
        true
    } else {
        to.parent().map(Path::exists).unwrap_or(true)
    };

    if !destination_dir_exists {
        return Err(ShellError::labeled_error(
            "Destination directory does not exist",
            "destination does not exist",
            to_tag,
        ));
    }

    let mut to = to.clone();
    if to.is_dir() {
        let from_file_name = match from.file_name() {
            Some(name) => name,
            None => {
                return Err(ShellError::labeled_error(
                    "Not a valid entry name",
                    "not a valid entry name",
                    from_tag,
                ))
            }
        };

        to.push(from_file_name);
    }

    move_item(&from, from_tag, &to)
}

fn move_item(from: &Path, from_tag: &Tag, to: &Path) -> Result<(), ShellError> {
    // We first try a rename, which is a quick operation. If that doesn't work, we'll try a copy
    // and remove the old file/folder. This is necessary if we're moving across filesystems or devices.
    std::fs::rename(&from, &to).or_else(|_| {
        match if from.is_file() {
            let mut options = fs_extra::file::CopyOptions::new();
            options.overwrite = true;
            fs_extra::file::move_file(from, to, &options)
        } else {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;
            fs_extra::dir::move_dir(from, to, &options)
        } {
            Ok(_) => Ok(()),
            Err(e) => Err(ShellError::labeled_error(
                format!("Could not move {:?} to {:?}. {:}", from, to, e.to_string()),
                "could not move",
                from_tag,
            )),
        }
    })
}

fn is_empty_dir(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(_) => true,
        Ok(mut s) => s.next().is_none(),
    }
}

fn is_hidden_dir(dir: impl AsRef<Path>) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        if let Ok(metadata) = dir.as_ref().metadata() {
            let attributes = metadata.file_attributes();
            // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
            (attributes & 0x2) != 0
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        dir.as_ref()
            .file_name()
            .map(|name| name.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }
}

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

pub fn get_file_type(md: &std::fs::Metadata) -> &str {
    let ft = md.file_type();
    let mut file_type = "Unknown";
    if ft.is_dir() {
        file_type = "Dir";
    } else if ft.is_file() {
        file_type = "File";
    } else if ft.is_symlink() {
        file_type = "Symlink";
    } else {
        #[cfg(unix)]
        {
            if ft.is_block_device() {
                file_type = "Block device";
            } else if ft.is_char_device() {
                file_type = "Char device";
            } else if ft.is_fifo() {
                file_type = "Pipe";
            } else if ft.is_socket() {
                file_type = "Socket";
            }
        }
    }
    file_type
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dir_entry_dict(
    filename: &std::path::Path,
    metadata: Option<&std::fs::Metadata>,
    tag: impl Into<Tag>,
    long: bool,
    short_name: bool,
    du: bool,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let mut dict = TaggedDictBuilder::new(&tag);
    // Insert all columns first to maintain proper table alignment if we can't find (or are not allowed to view) any information
    if long {
        #[cfg(windows)]
        {
            for column in [
                "name", "type", "target", "readonly", "size", "created", "accessed", "modified",
            ]
            .iter()
            {
                dict.insert_untagged(*column, UntaggedValue::nothing());
            }
        }

        #[cfg(unix)]
        {
            for column in [
                "name",
                "type",
                "target",
                "num_links",
                "inode",
                "readonly",
                "mode",
                "uid",
                "group",
                "size",
                "created",
                "accessed",
                "modified",
            ]
            .iter()
            {
                dict.insert_untagged(&(*column.to_owned()), UntaggedValue::nothing());
            }
        }
    } else {
        for column in ["name", "type", "target", "size", "modified"].iter() {
            if *column == "target" {
                continue;
            }
            dict.insert_untagged(*column, UntaggedValue::nothing());
        }
    }

    let name = if short_name {
        filename.file_name().and_then(|s| s.to_str())
    } else {
        filename.to_str()
    }
    .ok_or_else(|| {
        ShellError::labeled_error(
            format!("Invalid file name: {:}", filename.to_string_lossy()),
            "invalid file name",
            tag,
        )
    })?;

    dict.insert_untagged("name", UntaggedValue::filepath(name));

    if let Some(md) = metadata {
        dict.insert_untagged("type", get_file_type(md));
    }

    if long {
        if let Some(md) = metadata {
            if md.file_type().is_symlink() {
                let symlink_target_untagged_value: UntaggedValue;
                if let Ok(path_to_link) = filename.read_link() {
                    symlink_target_untagged_value =
                        UntaggedValue::string(path_to_link.to_string_lossy());
                } else {
                    symlink_target_untagged_value =
                        UntaggedValue::string("Could not obtain target file's path");
                }
                dict.insert_untagged("target", symlink_target_untagged_value);
            }
        }
    }

    if long {
        if let Some(md) = metadata {
            dict.insert_untagged(
                "readonly",
                UntaggedValue::boolean(md.permissions().readonly()),
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let mode = md.permissions().mode();
                dict.insert_untagged(
                    "mode",
                    UntaggedValue::string(umask::Mode::from(mode).to_string()),
                );

                let nlinks = md.nlink();
                dict.insert_untagged("num_links", UntaggedValue::string(nlinks.to_string()));

                let inode = md.ino();
                dict.insert_untagged("inode", UntaggedValue::string(inode.to_string()));

                if let Some(user) = users::get_user_by_uid(md.uid()) {
                    dict.insert_untagged(
                        "uid",
                        UntaggedValue::string(user.name().to_string_lossy()),
                    );
                }

                if let Some(group) = users::get_group_by_gid(md.gid()) {
                    dict.insert_untagged(
                        "group",
                        UntaggedValue::string(group.name().to_string_lossy()),
                    );
                }
            }
        }
    }

    if let Some(md) = metadata {
        let mut size_untagged_value: UntaggedValue = UntaggedValue::nothing();

        if md.is_dir() {
            let dir_size: u64 = if du {
                let params = DirBuilder::new(
                    Tag {
                        anchor: None,
                        span: Span::new(0, 2),
                    },
                    None,
                    false,
                    None,
                    false,
                );

                DirInfo::new(filename, &params, None, ctrl_c).get_size()
            } else {
                md.len()
            };

            size_untagged_value = UntaggedValue::filesize(dir_size);
        } else if md.is_file() {
            size_untagged_value = UntaggedValue::filesize(md.len());
        } else if md.file_type().is_symlink() {
            if let Ok(symlink_md) = filename.symlink_metadata() {
                size_untagged_value = UntaggedValue::filesize(symlink_md.len() as u64);
            }
        }

        dict.insert_untagged("size", size_untagged_value);
    }

    if let Some(md) = metadata {
        if long {
            if let Ok(c) = md.created() {
                dict.insert_untagged("created", UntaggedValue::system_date(c));
            }

            if let Ok(a) = md.accessed() {
                dict.insert_untagged("accessed", UntaggedValue::system_date(a));
            }
        }

        if let Ok(m) = md.modified() {
            dict.insert_untagged("modified", UntaggedValue::system_date(m));
        }
    }

    Ok(dict.into_value())
}

fn path_contains_hidden_folder(path: &Path, folders: &[PathBuf]) -> bool {
    let path_str = path.to_str().expect("failed to read path");
    if folders
        .iter()
        .any(|p| path_str.starts_with(&p.to_str().expect("failed to read hidden paths")))
    {
        return true;
    }
    false
}
