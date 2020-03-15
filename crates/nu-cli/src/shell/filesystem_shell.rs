use crate::commands::command::EvaluatedWholeStreamCommandArgs;
use crate::commands::cp::CopyArgs;
use crate::commands::ls::LsArgs;
use crate::commands::mkdir::MkdirArgs;
use crate::commands::mv::MoveArgs;
use crate::commands::rm::RemoveArgs;
use crate::data::dir_entry_dict;
use crate::prelude::*;
use crate::shell::completer::NuCompleter;
use crate::shell::shell::Shell;
use crate::utils::FileStructure;
use nu_errors::ShellError;
use nu_parser::ExpandContext;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue};
use rustyline::completion::FilenameCompleter;
use rustyline::hint::{Hinter, HistoryHinter};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::Ordering;
use trash as SendToTrash;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

    pub fn with_location(path: String, commands: CommandRegistry) -> FilesystemShell {
        let last_path = path.clone();
        FilesystemShell {
            path,
            last_path,
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
                homedir: dirs::home_dir(),
            },
            hinter: HistoryHinter {},
        }
    }

    fn canonicalize(&self, path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
        let path = if path.as_ref().is_relative() {
            let components = path.as_ref().components();
            let mut result = PathBuf::from(self.path());
            for component in components {
                match component {
                    Component::CurDir => { /* ignore current dir */ }
                    Component::ParentDir => {
                        result.pop();
                    }
                    Component::Normal(normal) => result.push(normal),
                    _ => {}
                }
            }

            result
        } else {
            path.as_ref().into()
        };

        dunce::canonicalize(path)
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
        }: LsArgs,
        context: &RunnablePerItemContext,
    ) -> Result<OutputStream, ShellError> {
        let ctrl_c = context.ctrl_c.clone();
        let name_tag = context.name.clone();

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
                    (PathBuf::from("./*"), context.name.clone())
                }
            }
        };

        let mut paths = glob::glob(&path.to_string_lossy())
            .map_err(|e| ShellError::labeled_error("Glob error", e.to_string(), &p_tag))?
            .peekable();

        if paths.peek().is_none() {
            return Err(ShellError::labeled_error(
                "Invalid File or Pattern",
                "invalid file or pattern",
                &p_tag,
            ));
        }

        // Generated stream: impl Stream<Item = Result<ReturnSuccess, ShellError>
        let stream = async_stream::try_stream! {
            for path in paths {
                // Handle CTRL+C presence
                if ctrl_c.load(Ordering::SeqCst) {
                    break;
                }

                // Map GlobError to ShellError and gracefully try to unwrap the path
                let path = path.map_err(|e| ShellError::from(e.into_error()))?;

                // Skip if '--all/-a' flag is present and this path is hidden
                if !all && is_hidden_dir(&path) {
                    continue;
                }

                // Get metadata from current path, if we don't have enough
                // permissions to stat on file don't use any metadata, otherwise
                // return the error and gracefully unwrap metadata (which yields
                // Option<Metadata>)
                let metadata = match std::fs::symlink_metadata(&path) {
                    Ok(metadata) => Ok(Some(metadata)),
                    Err(e) => if let PermissionDenied = e.kind() {
                        Ok(None)
                    } else {
                        Err(e)
                    },
                }?;

                // Build dict entry for this path and possibly using some metadata.
                // Map the possible dict entry into a Value, gracefully unwrap it
                // with '?'
                let entry = dir_entry_dict(
                    &path,
                    metadata.as_ref(),
                    name_tag.clone(),
                    full,
                    short_names,
                    with_symlink_targets
                )
                .map(|entry| ReturnSuccess::Value(entry.into()))?;

                // Finally yield the generated entry that was mapped to Value
                yield entry;
            }
        };

        Ok(stream.to_output_stream())
    }

    fn cd(&self, args: EvaluatedWholeStreamCommandArgs) -> Result<OutputStream, ShellError> {
        let path = match args.nth(0) {
            None => match dirs::home_dir() {
                Some(o) => o,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Cannot change to home directory",
                        "cannot go to home",
                        &args.call_info.name_tag,
                    ))
                }
            },
            Some(v) => {
                let target = v.as_path()?;

                if target == Path::new("-") {
                    PathBuf::from(&self.last_path)
                } else {
                    let path = self.canonicalize(target).map_err(|_| {
                        ShellError::labeled_error(
                            "Cannot change to directory",
                            "directory not found",
                            &v.tag,
                        )
                    })?;

                    if !path.is_dir() {
                        return Err(ShellError::labeled_error(
                            "Cannot change to directory",
                            "is not a directory",
                            &v.tag,
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
                                    &v.tag,
                                )
                            })?;

                        if !has_exec {
                            return Err(ShellError::labeled_error(
                                "Cannot change to directory",
                                "permission denied",
                                &v.tag,
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

        let mut source = PathBuf::from(path);
        let mut destination = PathBuf::from(path);

        source.push(&src.item);
        destination.push(&dst.item);

        let sources: Vec<_> = match glob::glob(&source.to_string_lossy()) {
            Ok(files) => files.collect(),
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "Invalid pattern",
                    "invalid pattern",
                    src.tag,
                ))
            }
        };

        if sources.len() == 1 {
            if let Ok(entry) = &sources[0] {
                if entry.is_dir() && !recursive.item {
                    return Err(ShellError::labeled_error(
                        "is a directory (not copied). Try using \"--recursive\".",
                        "is a directory (not copied). Try using \"--recursive\".",
                        src.tag,
                    ));
                }

                let mut sources: FileStructure = FileStructure::new();

                sources.walk_decorate(&entry)?;

                if entry.is_file() {
                    let strategy = |(source_file, _depth_level)| {
                        if destination.is_dir() {
                            let mut new_dst = dunce::canonicalize(destination.clone())?;
                            if let Some(name) = entry.file_name() {
                                new_dst.push(name);
                            }
                            Ok((source_file, new_dst))
                        } else {
                            Ok((source_file, destination.clone()))
                        }
                    };

                    let sources = sources.paths_applying_with(strategy)?;

                    for (ref src, ref dst) in sources {
                        if src.is_file() {
                            match std::fs::copy(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        e.to_string(),
                                        e.to_string(),
                                        name_tag,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }
                }

                if entry.is_dir() {
                    if !destination.exists() {
                        match std::fs::create_dir_all(&destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    dst.tag,
                                ));
                            }
                            Ok(o) => o,
                        };

                        let strategy = |(source_file, depth_level)| {
                            let mut new_dst = destination.clone();
                            let path = dunce::canonicalize(&source_file)?;

                            let mut comps: Vec<_> = path
                                .components()
                                .map(|fragment| fragment.as_os_str())
                                .rev()
                                .take(1 + depth_level)
                                .collect();

                            comps.reverse();

                            for fragment in comps.iter() {
                                new_dst.push(fragment);
                            }

                            Ok((PathBuf::from(&source_file), new_dst))
                        };

                        let sources = sources.paths_applying_with(strategy)?;

                        let dst_tag = dst.tag;
                        for (ref src, ref dst) in sources {
                            if src.is_dir() && !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            dst_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }

                            if src.is_file() {
                                match std::fs::copy(src, dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }
                    } else {
                        match entry.file_name() {
                            Some(name) => destination.push(name),
                            None => {
                                return Err(ShellError::labeled_error(
                                    "Copy aborted. Not a valid path",
                                    "not a valid path",
                                    dst.tag,
                                ))
                            }
                        }

                        match std::fs::create_dir_all(&destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    dst.tag,
                                ));
                            }
                            Ok(o) => o,
                        };

                        let strategy = |(source_file, depth_level)| {
                            let mut new_dst = dunce::canonicalize(&destination)?;
                            let path = dunce::canonicalize(&source_file)?;

                            let mut comps: Vec<_> = path
                                .components()
                                .map(|fragment| fragment.as_os_str())
                                .rev()
                                .take(1 + depth_level)
                                .collect();

                            comps.reverse();

                            for fragment in comps.iter() {
                                new_dst.push(fragment);
                            }

                            Ok((PathBuf::from(&source_file), new_dst))
                        };

                        let sources = sources.paths_applying_with(strategy)?;

                        let dst_tag = dst.tag;
                        for (ref src, ref dst) in sources {
                            if src.is_dir() && !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            dst_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }

                            if src.is_file() {
                                match std::fs::copy(src, dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            e.to_string(),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }
                    }
                }
            }
        } else if destination.exists() {
            if !sources.iter().all(|x| match x {
                Ok(f) => f.is_file(),
                Err(_) => false,
            }) {
                return Err(ShellError::labeled_error(
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
                    "recursive copying in patterns not supported",
                    src.tag,
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let mut to = PathBuf::from(&destination);

                    match entry.file_name() {
                        Some(name) => to.push(name),
                        None => {
                            return Err(ShellError::labeled_error(
                                "Copy aborted. Not a valid path",
                                "not a valid path",
                                dst.tag,
                            ))
                        }
                    }

                    if entry.is_file() {
                        match std::fs::copy(&entry, &to) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    src.tag,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }
        } else {
            let destination_file_name = {
                match destination.file_name() {
                    Some(name) => PathBuf::from(name),
                    None => {
                        return Err(ShellError::labeled_error(
                            "Copy aborted. Not a valid destination",
                            "not a valid destination",
                            dst.tag,
                        ))
                    }
                }
            };

            return Err(ShellError::labeled_error(
                format!("Copy aborted. (Does {:?} exist?)", destination_file_name),
                format!("copy aborted (does {:?} exist?)", destination_file_name),
                dst.tag,
            ));
        }

        Ok(OutputStream::empty())
    }

    fn mkdir(
        &self,
        MkdirArgs { rest: directories }: MkdirArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let full_path = PathBuf::from(path);

        if directories.is_empty() {
            return Err(ShellError::labeled_error(
                "mkdir requires directory paths",
                "needs parameter",
                name,
            ));
        }

        for dir in directories.iter() {
            let create_at = {
                let mut loc = full_path.clone();
                loc.push(&dir.item);
                loc
            };

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
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let name_tag = name;

        let mut source = PathBuf::from(path);
        let mut destination = PathBuf::from(path);

        source.push(&src.item);
        destination.push(&dst.item);

        let sources: Vec<_> = match glob::glob(&source.to_string_lossy()) {
            Ok(files) => files.collect(),
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "Invalid pattern.",
                    "invalid pattern",
                    src.tag,
                ))
            }
        };

        if sources.is_empty() {
            return Err(ShellError::labeled_error(
                "Invalid File or Pattern.",
                "Invalid File or Pattern",
                src.tag,
            ));
        }
        let destination_file_name = {
            match destination.file_name() {
                Some(name) => PathBuf::from(name),
                None => {
                    return Err(ShellError::labeled_error(
                        "Rename aborted. Not a valid destination",
                        "not a valid destination",
                        dst.tag,
                    ))
                }
            }
        };

        if sources.is_empty() {
            return Err(ShellError::labeled_error(
                "Move aborted. Not a valid destination",
                "not a valid destination",
                src.tag,
            ));
        }

        if sources.len() == 1 {
            if let Ok(entry) = &sources[0] {
                let entry_file_name = match entry.file_name() {
                    Some(name) => name,
                    None => {
                        return Err(ShellError::labeled_error(
                            "Rename aborted. Not a valid entry name",
                            "not a valid entry name",
                            src.tag,
                        ))
                    }
                };

                if destination.exists() && destination.is_dir() {
                    destination = match dunce::canonicalize(&destination) {
                        Ok(path) => path,
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                format!("Rename aborted. {:}", e.to_string()),
                                e.to_string(),
                                dst.tag,
                            ))
                        }
                    };

                    destination.push(entry_file_name);
                }

                if entry.is_file() {
                    #[cfg(not(windows))]
                    {
                        match std::fs::rename(&entry, &destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
                                    e.to_string(),
                                    name_tag,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                    #[cfg(windows)]
                    {
                        match std::fs::copy(&entry, &destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
                                    e.to_string(),
                                    name_tag,
                                ));
                            }
                            Ok(_) => match std::fs::remove_file(&entry) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
                                        e.to_string(),
                                        name_tag,
                                    ));
                                }
                                Ok(o) => o,
                            },
                        };
                    }
                }

                if entry.is_dir() {
                    match std::fs::create_dir_all(&destination) {
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                format!(
                                    "Rename {:?} to {:?} aborted. {:}",
                                    entry_file_name,
                                    destination_file_name,
                                    e.to_string(),
                                ),
                                e.to_string(),
                                name_tag,
                            ));
                        }
                        Ok(o) => o,
                    };
                    #[cfg(not(windows))]
                    {
                        match std::fs::rename(&entry, &destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
                                    e.to_string(),
                                    name_tag,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                    #[cfg(windows)]
                    {
                        let mut sources: FileStructure = FileStructure::new();

                        sources.walk_decorate(&entry)?;

                        let strategy = |(source_file, depth_level)| {
                            let mut new_dst = destination.clone();

                            let path = dunce::canonicalize(&source_file)?;

                            let mut comps: Vec<_> = path
                                .components()
                                .map(|fragment| fragment.as_os_str())
                                .rev()
                                .take(1 + depth_level)
                                .collect();

                            comps.reverse();

                            for fragment in comps.iter() {
                                new_dst.push(fragment);
                            }

                            Ok((PathBuf::from(&source_file), new_dst))
                        };

                        let sources = sources.paths_applying_with(strategy)?;

                        for (ref src, ref dst) in sources {
                            if src.is_dir() && !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                }
                            } else if src.is_file() {
                                match std::fs::copy(src, dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            format!(
                                                "Moving file {:?} to {:?} aborted. {:}",
                                                src,
                                                dst,
                                                e.to_string(),
                                            ),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(_o) => (),
                                }
                            }
                        }

                        if src.is_file() {
                            match std::fs::copy(&src, &dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            src,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
                                        e.to_string(),
                                        name_tag,
                                    ));
                                }
                                Ok(_) => match std::fs::remove_file(&src) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                },
                            };
                        }

                        match std::fs::remove_dir_all(entry) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
                                    e.to_string(),
                                    name_tag,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }
        } else if destination.exists() {
            let is_file = |x: &Result<PathBuf, _>| {
                x.as_ref().map(|entry| entry.is_file()).unwrap_or_default()
            };

            if !sources.iter().all(is_file) {
                return Err(ShellError::labeled_error(
                    "Rename aborted (directories found). Renaming in patterns not supported yet (try moving the directory directly)",
                    "renaming in patterns not supported yet (try moving the directory directly)",
                    src.tag,
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let entry_file_name = match entry.file_name() {
                        Some(name) => name,
                        None => {
                            return Err(ShellError::labeled_error(
                                "Rename aborted. Not a valid entry name",
                                "not a valid entry name",
                                src.tag,
                            ))
                        }
                    };

                    let mut to = PathBuf::from(&destination);
                    to.push(entry_file_name);

                    if entry.is_file() {
                        #[cfg(not(windows))]
                        {
                            match std::fs::rename(&entry, &to) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
                                        e.to_string(),
                                        name_tag,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                        #[cfg(windows)]
                        {
                            match std::fs::copy(&entry, &to) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
                                        e.to_string(),
                                        name_tag,
                                    ));
                                }
                                Ok(_) => match std::fs::remove_file(&entry) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            format!(
                                                "Remove {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
                                            e.to_string(),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
                                },
                            };
                        }
                    }
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                format!("Rename aborted. (Does {:?} exist?)", destination_file_name),
                format!("rename aborted (does {:?} exist?)", destination_file_name),
                dst.tag,
            ));
        }

        Ok(OutputStream::empty())
    }

    fn rm(
        &self,
        RemoveArgs {
            target,
            recursive,
            trash,
        }: RemoveArgs,
        name: Tag,
        path: &str,
    ) -> Result<OutputStream, ShellError> {
        let name_tag = name;

        if target.item.to_str() == Some(".") || target.item.to_str() == Some("..") {
            return Err(ShellError::labeled_error(
                "Remove aborted. \".\" or \"..\" may not be removed.",
                "\".\" or \"..\" may not be removed",
                target.tag,
            ));
        }

        let mut path = PathBuf::from(path);

        path.push(&target.item);

        match glob::glob(&path.to_string_lossy()) {
            Ok(files) => {
                let files: Vec<_> = files.collect();
                if files.is_empty() {
                    Err(ShellError::labeled_error(
                        "Remove aborted. Not a valid path",
                        "not a valid path",
                        target.tag,
                    ))
                } else {
                    let stream = async_stream! {
                        for file in files.iter() {
                            match file {
                                Ok(f) => {
                                    let is_empty =  match f.read_dir() {
                                            Ok(mut p) => p.next().is_none(),
                                            Err(_) => false
                                    };

                                    let valid_target =
                                        f.exists() && (!f.is_dir() || (is_empty || recursive.item));
                                    if valid_target {
                                        if trash.item {
                                            match SendToTrash::remove(f) {
                                                Err(e) => {
                                                    let msg = format!(
                                                        "Could not delete {:}",
                                                        f.to_string_lossy()
                                                    );
                                                    let label = format!("{:?}", e);
                                                    yield Err(ShellError::labeled_error(
                                                        msg,
                                                        label,
                                                        &target.tag,
                                                    ))
                                                },
                                                Ok(()) => {
                                                    let val = format!("deleted {:}", f.to_string_lossy()).into();
                                                    yield Ok(ReturnSuccess::Value(val))
                                                },
                                            }
                                        } else {
                                            let success = if f.is_dir() {
                                                std::fs::remove_dir_all(f)
                                            } else {
                                                std::fs::remove_file(f)
                                            };
                                            match success {
                                                Err(e) => {
                                                    let msg = format!(
                                                        "Could not delete {:}",
                                                        f.to_string_lossy()
                                                    );
                                                    yield Err(ShellError::labeled_error(
                                                        msg,
                                                        e.to_string(),
                                                        &target.tag,
                                                    ))
                                                },
                                                Ok(()) => {
                                                    let val = format!("deleted {:}", f.to_string_lossy()).into();
                                                    yield Ok(ReturnSuccess::Value(
                                                        val,
                                                    ))
                                                },
                                            }
                                        }
                                    } else {
                                        if f.is_dir() {
                                            let msg = format!(
                                                "Cannot remove {:}. try --recursive",
                                                f.to_string_lossy()
                                            );
                                            yield Err(ShellError::labeled_error(
                                                msg,
                                                "cannot remove non-empty directory",
                                                &target.tag,
                                            ))
                                        } else {
                                            let msg = format!("Invalid file: {:}", f.to_string_lossy());
                                            yield Err(ShellError::labeled_error(
                                                msg,
                                                "invalid file",
                                                &target.tag,
                                            ))
                                        }
                                    }
                                }
                                Err(e) => {
                                    let msg = format!("Could not remove {:}", path.to_string_lossy());
                                    yield Err(ShellError::labeled_error(
                                        msg,
                                        e.to_string(),
                                        &target.tag,
                                    ))
                                },
                            }
                            }
                    };
                    Ok(stream.to_output_stream())
                }
            }
            Err(e) => Err(ShellError::labeled_error(
                format!("Remove aborted. {:}", e.to_string()),
                e.to_string(),
                &name_tag,
            )),
        }
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
        let path = match dunce::canonicalize(pathbuf.as_path()) {
            Ok(path) => {
                let _ = std::env::set_current_dir(&path);
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
        self.completer.complete(line, pos, ctx)
    }

    fn hint(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
        _expand_context: ExpandContext,
    ) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
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
