use crate::commands::cd::CdArgs;
use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::data::dir_entry_dict;
use crate::path::canonicalize;
use crate::prelude::*;
use crate::shell::completer::NuCompleter;
use crate::shell::shell::Shell;
use crate::utils::FileStructure;

use rustyline::completion::FilenameCompleter;
use rustyline::hint::{Hinter, HistoryHinter};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use nu_errors::ShellError;
use nu_parser::expand_ndots;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue};
use nu_source::Tagged;

pub struct FilesystemShell {
    pub(crate) path: String,
    pub(crate) last_path: String,
    completer: NuCompleter,
    hinter: HistoryHinter,
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
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands: self.completer.commands.clone(),
                homedir: self.homedir(),
            },
            hinter: HistoryHinter {},
        }
    }
}

impl FilesystemShell {
    pub fn basic(commands: CommandRegistry) -> Result<FilesystemShell, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(FilesystemShell {
            path: path.to_string_lossy().to_string(),
            last_path: path.to_string_lossy().to_string(),
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
                homedir: dirs::home_dir(),
            },
            hinter: HistoryHinter {},
        })
    }

    pub fn with_location(
        path: String,
        commands: CommandRegistry,
    ) -> Result<FilesystemShell, std::io::Error> {
        let path = canonicalize(std::env::current_dir()?, &path)?;
        let path = path.display().to_string();
        let last_path = path.clone();

        Ok(FilesystemShell {
            path,
            last_path,
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
                homedir: dirs::home_dir(),
            },
            hinter: HistoryHinter {},
        })
    }
}

impl Shell for FilesystemShell {
    fn name(&self) -> String {
        "filesystem".to_string()
    }

    fn homedir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn ls(
        &self,
        LsArgs {
            path,
            all,
            full,
            short_names,
            with_symlink_targets,
            du,
        }: LsArgs,
        name_tag: Tag,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<OutputStream, ShellError> {
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

        // Generated stream: impl Stream<Item = Result<ReturnSuccess, ShellError>
        let stream = async_stream::try_stream! {
            for path in paths {
                let path = path.map_err(|e| ShellError::from(e.into_error()))?;

                if !all && is_hidden_dir(&path) {
                    continue;
                }

                let metadata = match std::fs::symlink_metadata(&path) {
                    Ok(metadata) => Ok(Some(metadata)),
                    Err(e) => if let PermissionDenied = e.kind() {
                        Ok(None)
                    } else {
                        Err(e)
                    },
                }?;

                let entry = dir_entry_dict(
                    &path,
                    metadata.as_ref(),
                    name_tag.clone(),
                    full,
                    short_names,
                    with_symlink_targets,
                    du,
                )
                .map(|entry| ReturnSuccess::Value(entry.into()))?;

                yield entry;
            }
        };

        Ok(stream.interruptible(ctrl_c).to_output_stream())
    }

    fn cd(&self, args: CdArgs, name: Tag) -> Result<OutputStream, ShellError> {
        let path = match args.path {
            None => match dirs::home_dir() {
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

        let any_source_is_dir = sources.iter().any(|f| match f {
            Ok(f) => f.is_dir(),
            Err(_) => false,
        });

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
        MkdirArgs { rest: directories }: MkdirArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let path = Path::new(path);

        if directories.is_empty() {
            return Err(ShellError::labeled_error(
                "mkdir requires directory paths",
                "needs parameter",
                name,
            ));
        }

        for dir in directories.iter() {
            let create_at = path.join(&dir.item);

            let dir_res = std::fs::create_dir_all(create_at);
            if let Err(reason) = dir_res {
                return Err(ShellError::labeled_error(
                    reason.to_string(),
                    reason.to_string(),
                    dir.tag(),
                ));
            }
        }

        Ok(OutputStream::empty())
    }

    fn mv(
        &self,
        MoveArgs { src, dst }: MoveArgs,
        _name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let path = Path::new(path);
        let source = path.join(&src.item);
        let destination = path.join(&dst.item);

        let sources =
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

        if all_targets.is_empty() {
            return Err(ShellError::labeled_error(
                "No valid paths",
                "no valid paths",
                name_tag,
            ));
        }

        let stream = async_stream! {
            for (f, tag) in all_targets.iter() {
                let is_empty = || match f.read_dir() {
                    Ok(mut p) => p.next().is_none(),
                    Err(_) => false
                };

                if let Ok(metadata) = f.symlink_metadata() {
                    if metadata.is_file() || metadata.file_type().is_symlink() || recursive.item || is_empty() {
                        let result;
                        #[cfg(feature = "trash-support")]
                        {
                            result = if _trash.item {
                                trash::remove(f)
                                   .map_err(|e| f.to_string_lossy())
                            } else if metadata.is_file() {
                                std::fs::remove_file(f)
                                    .map_err(|e| f.to_string_lossy())
                            } else {
                                std::fs::remove_dir_all(f)
                                    .map_err(|e| f.to_string_lossy())
                            };
                        }
                        #[cfg(not(feature = "trash-support"))]
                        {
                            result = if metadata.is_file() {
                                std::fs::remove_file(f)
                                    .map_err(|e| f.to_string_lossy())
                            } else {
                                std::fs::remove_dir_all(f)
                                    .map_err(|e| f.to_string_lossy())
                            };
                        }

                        if let Err(e) = result {
                            let msg = format!("Could not delete {:}", e);
                            yield Err(ShellError::labeled_error(msg, e, tag))
                        } else {
                            let val = format!("deleted {:}", f.to_string_lossy()).into();
                            yield Ok(ReturnSuccess::Value(val))
                        }
                    } else {
                        let msg = format!(
                            "Cannot remove {:}. try --recursive",
                            f.to_string_lossy()
                        );
                        yield Err(ShellError::labeled_error(
                            msg,
                            "cannot remove non-empty directory",
                            tag,
                        ))
                    }
                } else {
                    let msg = format!("no such file or directory: {:}", f.to_string_lossy());
                    yield Err(ShellError::labeled_error(
                        msg,
                        "no such file or directory",
                        tag,
                    ))
                }
            }
        };

        Ok(stream.to_output_stream())
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

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), rustyline::error::ReadlineError> {
        let expanded = expand_ndots(&line);

        // Find the first not-matching char position, if there is one
        let differ_pos = line
            .chars()
            .zip(expanded.chars())
            .enumerate()
            .find(|(_index, (a, b))| a != b)
            .map(|(differ_pos, _)| differ_pos);

        let pos = if let Some(differ_pos) = differ_pos {
            if differ_pos < pos {
                pos + (expanded.len() - line.len())
            } else {
                pos
            }
        } else {
            pos
        };

        self.completer.complete(&expanded, pos, ctx)
    }

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
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

    // We first try a rename, which is a quick operation. If that doesn't work, we'll try a copy
    // and remove the old file. This is necessary if we're moving across filesystems.
    std::fs::rename(&from, &to)
        .or_else(|_| std::fs::copy(&from, &to).and_then(|_| std::fs::remove_file(&from)))
        .map_err(|e| {
            ShellError::labeled_error(
                format!("Could not move {:?} to {:?}. {:}", from, to, e.to_string()),
                "could not move",
                from_tag,
            )
        })
}

fn is_empty_dir(dir: impl AsRef<Path>) -> bool {
    match dir.as_ref().read_dir() {
        Err(_) => true,
        Ok(mut s) => s.next().is_none(),
    }
}

fn is_hidden_dir(dir: impl AsRef<Path>) -> bool {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;

            if let Ok(metadata) = dir.as_ref().metadata() {
                let attributes = metadata.file_attributes();
                // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
                (attributes & 0x2) != 0
            } else {
                false
            }
        } else {
            dir.as_ref()
                .file_name()
                .map(|name| name.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
        }
    }
}
