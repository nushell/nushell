use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::PathBuf;

#[cfg(windows)]
use crate::utils::FileStructure;

pub struct Move;

impl PerItemCommand for Move {
    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        shell_manager: &ShellManager,
        _input: Tagged<Value>,
    ) -> Result<VecDeque<ReturnValue>, ShellError> {
        mv(call_info, shell_manager)
    }

    fn name(&self) -> &str {
        "mv"
    }

    fn signature(&self) -> Signature {
        Signature::build("mv").named("file", SyntaxType::Any)
    }
}

pub fn mv(
    call_info: &CallInfo,
    shell_manager: &ShellManager,
) -> Result<VecDeque<ReturnValue>, ShellError> {
    let mut source = PathBuf::from(shell_manager.path());
    let mut destination = PathBuf::from(shell_manager.path());
    let span = call_info.name_span;

    match call_info
        .args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            source.push(file);
        }
    }

    match call_info
        .args
        .nth(1)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            destination.push(file);
        }
    }

    let sources = glob::glob(&source.to_string_lossy());

    if sources.is_err() {
        return Err(ShellError::labeled_error(
            "Invalid pattern.",
            "Invalid pattern.",
            call_info.args.nth(0).unwrap().span(),
        ));
    }

    let sources: Vec<_> = sources.unwrap().collect();

    if sources.len() == 1 {
        if let Ok(entry) = &sources[0] {
            if destination.exists() && destination.is_dir() {
                destination = dunce::canonicalize(&destination).unwrap();
                destination.push(source.file_name().unwrap());
            }

            if entry.is_file() {
                match std::fs::rename(&entry, &destination) {
                    Err(e) => {
                        return Err(ShellError::labeled_error(
                            format!(
                                "Rename {:?} to {:?} aborted. {:}",
                                entry.file_name().unwrap(),
                                destination.file_name().unwrap(),
                                e.to_string(),
                            ),
                            format!(
                                "Rename {:?} to {:?} aborted. {:}",
                                entry.file_name().unwrap(),
                                destination.file_name().unwrap(),
                                e.to_string(),
                            ),
                            span,
                        ));
                    }
                    Ok(o) => o,
                };
            }

            if entry.is_dir() {
                match std::fs::create_dir_all(&destination) {
                    Err(e) => {
                        return Err(ShellError::labeled_error(
                            format!(
                                "Rename {:?} to {:?} aborted. {:}",
                                entry.file_name().unwrap(),
                                destination.file_name().unwrap(),
                                e.to_string(),
                            ),
                            format!(
                                "Rename {:?} to {:?} aborted. {:}",
                                entry.file_name().unwrap(),
                                destination.file_name().unwrap(),
                                e.to_string(),
                            ),
                            span,
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
                                    entry.file_name().unwrap(),
                                    destination.file_name().unwrap(),
                                    e.to_string(),
                                ),
                                format!(
                                    "Rename {:?} to {:?} aborted. {:}",
                                    entry.file_name().unwrap(),
                                    destination.file_name().unwrap(),
                                    e.to_string(),
                                ),
                                span,
                            ));
                        }
                        Ok(o) => o,
                    };
                }
                #[cfg(windows)]
                {
                    let mut sources: FileStructure = FileStructure::new();

                    sources.walk_decorate(&entry);

                    let strategy = |(source_file, depth_level)| {
                        let mut new_dst = destination.clone();
                        let path = dunce::canonicalize(&source_file).unwrap();

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

                        (PathBuf::from(&source_file), PathBuf::from(new_dst))
                    };

                    for (ref src, ref dst) in sources.paths_applying_with(strategy) {
                        if src.is_dir() {
                            if !dst.exists() {
                                match std::fs::create_dir_all(dst) {
                                    Err(e) => {
                                        return Err(ShellError::labeled_error(
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry.file_name().unwrap(),
                                                destination.file_name().unwrap(),
                                                e.to_string(),
                                            ),
                                            format!(
                                                "Rename {:?} to {:?} aborted. {:}",
                                                entry.file_name().unwrap(),
                                                destination.file_name().unwrap(),
                                                e.to_string(),
                                            ),
                                            span,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }

                        if src.is_file() {
                            match std::fs::rename(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry.file_name().unwrap(),
                                            destination.file_name().unwrap(),
                                            e.to_string(),
                                        ),
                                        format!(
                                            "Rename {:?} to {:?} aborted. {:}",
                                            entry.file_name().unwrap(),
                                            destination.file_name().unwrap(),
                                            e.to_string(),
                                        ),
                                        span,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }

                    std::fs::remove_dir_all(entry).expect("can not remove directory");
                }
            }
        }
    } else {
        if destination.exists() {
            if !sources.iter().all(|x| (x.as_ref().unwrap()).is_file()) {
                return Err(ShellError::labeled_error(
                    "Rename aborted (directories found). Renaming in patterns not supported yet (try moving the directory directly)",
                    "Rename aborted (directories found). Renaming in patterns not supported yet (try moving the directory directly)",
                    call_info.args.nth(0).unwrap().span(),
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let mut to = PathBuf::from(&destination);
                    to.push(&entry.file_name().unwrap());

                    if entry.is_file() {
                        match std::fs::rename(&entry, &to) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry.file_name().unwrap(),
                                        destination.file_name().unwrap(),
                                        e.to_string(),
                                    ),
                                    format!(
                                        "Rename {:?} to {:?} aborted. {:}",
                                        entry.file_name().unwrap(),
                                        destination.file_name().unwrap(),
                                        e.to_string(),
                                    ),
                                    span,
                                ));
                            }
                            Ok(o) => o,
                        };
                    }
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                format!(
                    "Rename aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                format!(
                    "Rename aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                call_info.args.nth(1).unwrap().span(),
            ));
        }
    }

    Ok(VecDeque::new())
}
