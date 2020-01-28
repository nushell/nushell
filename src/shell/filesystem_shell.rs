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
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use trash as SendToTrash;

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
                    if is_dir_empty(&p) {
                        return Ok(OutputStream::empty());
                    }
                    p.push("*");
                }
                (p, p_tag)
            }
            None => {
                if is_dir_empty(&self.path().into()) {
                    return Ok(OutputStream::empty());
                } else {
                    (PathBuf::from("./*"), context.name.clone())
                }
            }
        };

        let mut paths = match glob::glob(&path.to_string_lossy()) {
            Ok(g) => Ok(g),
            Err(e) => Err(ShellError::labeled_error("Glob error", e.msg, &p_tag)),
        }?
        .peekable();

        if paths.peek().is_none() {
            return Err(ShellError::labeled_error(
                "Invalid File or Pattern",
                "Invalid File or Pattern",
                &p_tag,
            ));
        }

        let stream = async_stream! {
            for path in paths {
                if ctrl_c.load(Ordering::SeqCst) {
                    break;
                }
                match path {
                    Ok(p) => match std::fs::symlink_metadata(&p) {
                        Ok(m) => {
                            match dir_entry_dict(&p, &m, name_tag.clone(), full, short_names, with_symlink_targets) {
                                Ok(d) => yield ReturnSuccess::value(d),
                                Err(e) => yield Err(e)
                            }
                        }
                        Err(e) => yield Err(ShellError::from(e))
                    }
                    Err(e) => yield Err(e.into_error().into()),
                }
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
                        "Can not change to home directory",
                        "can not go to home",
                        &args.call_info.name_tag,
                    ))
                }
            },
            Some(v) => {
                let target = v.as_path()?;

                if PathBuf::from("-") == target {
                    PathBuf::from(&self.last_path)
                } else {
                    let path = PathBuf::from(self.path());

                    if target.exists() && !target.is_dir() {
                        return Err(ShellError::labeled_error(
                            "Can not change to directory",
                            "is not a directory",
                            v.tag(),
                        ));
                    }

                    match dunce::canonicalize(path.join(&target)) {
                        Ok(p) => p,
                        Err(_) => {
                            return Err(ShellError::labeled_error(
                                "Can not change to directory",
                                "directory not found",
                                v.tag(),
                            ))
                        }
                    }
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
                    "Invalid pattern.",
                    "Invalid pattern.",
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
                        if destination.exists() {
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
                                    name_tag,
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

                        for (ref src, ref dst) in sources {
                            if src.is_dir() && !dst.exists() {
                                match std::fs::create_dir_all(dst) {
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
                                    "Copy aborted. Not a valid path",
                                    name_tag,
                                ))
                            }
                        }

                        match std::fs::create_dir_all(&destination) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    name_tag,
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

                        for (ref src, ref dst) in sources {
                            if src.is_dir() && !dst.exists() {
                                match std::fs::create_dir_all(dst) {
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
                    "Copy aborted (directories found). Recursive copying in patterns not supported yet (try copying the directory directly)",
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
                                "Copy aborted. Not a valid path",
                                name_tag,
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
                            "Copy aborted. Not a valid destination",
                            name_tag,
                        ))
                    }
                }
            };

            return Err(ShellError::labeled_error(
                format!("Copy aborted. (Does {:?} exist?)", destination_file_name),
                format!("Copy aborted. (Does {:?} exist?)", destination_file_name),
                dst.tag(),
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
                    "Invalid pattern.",
                    src.tag,
                ))
            }
        };

        let destination_file_name = {
            match destination.file_name() {
                Some(name) => PathBuf::from(name),
                None => {
                    return Err(ShellError::labeled_error(
                        "Rename aborted. Not a valid destination",
                        "Rename aborted. Not a valid destination",
                        dst.tag(),
                    ))
                }
            }
        };

        if sources.len() == 1 {
            if let Ok(entry) = &sources[0] {
                let entry_file_name = match entry.file_name() {
                    Some(name) => name,
                    None => {
                        return Err(ShellError::labeled_error(
                            "Rename aborted. Not a valid entry name",
                            "Rename aborted. Not a valid entry name",
                            name_tag,
                        ))
                    }
                };

                if destination.exists() && destination.is_dir() {
                    destination = match dunce::canonicalize(&destination) {
                        Ok(path) => path,
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                format!("Rename aborted. {:}", e.to_string()),
                                format!("Rename aborted. {:}", e.to_string()),
                                name_tag,
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
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
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
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
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
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
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
                                format!(
                                    "Rename {:?} to {:?} aborted. {:}",
                                    entry_file_name,
                                    destination_file_name,
                                    e.to_string(),
                                ),
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
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
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
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
                                            name_tag,
                                        ));
                                    }
                                    Ok(o) => o,
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
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            src,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
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
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
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
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry_file_name,
                                        destination_file_name,
                                        e.to_string(),
                                    ),
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
                    "Rename aborted (directories found). Renaming in patterns not supported yet (try moving the directory directly)",
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
                                "Rename aborted. Not a valid entry name",
                                name_tag,
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
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
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
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry_file_name,
                                            destination_file_name,
                                            e.to_string(),
                                        ),
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
                                            format!(
                                                "Remove {:?} to {:?} aborted. {:}",
                                                entry_file_name,
                                                destination_file_name,
                                                e.to_string(),
                                            ),
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
                format!("Rename aborted. (Does {:?} exist?)", destination_file_name),
                dst.tag(),
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
                "Remove aborted. \".\" or \"..\" may not be removed.",
                target.tag(),
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
                        "Remove aborted. Not a valid path",
                        &name_tag,
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
                                        f.is_file() || (f.is_dir() && (is_empty || recursive.item));
                                    if valid_target {
                                        if trash.item {
                                            match SendToTrash::remove(f) {
                                                Err(e) => {
                                                    let msg = format!(
                                                        "Could not delete {:}",
                                                        f.to_string_lossy()
                                                    );
                                                    let label = format!("Error {:?}", e);
                                                    yield Err(ShellError::labeled_error(
                                                        msg,
                                                        label,
                                                        &name_tag,
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
                                                    let label = format!("Error {:}", e.to_string());
                                                    yield Err(ShellError::labeled_error(
                                                        msg,
                                                        label,
                                                        &name_tag,
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
                                            let label = format!("Cannot remove non-empty directory");
                                            yield Err(ShellError::labeled_error(
                                                msg,
                                                label,
                                                &name_tag,
                                            ))
                                        } else {
                                            let msg = format!("Invalid file: {:}", f.to_string_lossy());
                                            let label = format!("Invalid file");
                                            yield Err(ShellError::labeled_error(
                                                msg,
                                                label,
                                                &name_tag,
                                            ))
                                        }
                                    }
                                }
                                Err(e) => {
                                    let msg = format!("Could not remove {:}", path.to_string_lossy());
                                    let label = format!("Error {:}", e.to_string());
                                    yield Err(ShellError::labeled_error(
                                        msg,
                                        label,
                                        &name_tag,
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
                format!("Remove aborted. {:}", e.to_string()),
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

fn is_dir_empty(d: &PathBuf) -> bool {
    match d.read_dir() {
        Err(_e) => true,
        Ok(mut s) => s.next().is_none(),
    }
}
