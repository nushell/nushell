use crate::errors::ShellError;
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub struct Copycp;

impl StaticCommand for Copycp {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cp(args, registry)
    }

    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .named("file", SyntaxType::Any)
            .switch("recursive")
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Res {
    pub loc: PathBuf,
    pub at: usize,
}

impl Res {}

pub struct FileStructure {
    root: PathBuf,
    resources: Vec<Res>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure {
            root: PathBuf::new(),
            resources: Vec::<Res>::new(),
        }
    }

    pub fn set_root(&mut self, path: &Path) {
        self.root = path.to_path_buf();
    }

    pub fn paths_applying_with<F>(&mut self, to: F) -> Vec<(PathBuf, PathBuf)>
    where
        F: Fn((PathBuf, usize)) -> (PathBuf, PathBuf),
    {
        self.resources
            .iter()
            .map(|f| (PathBuf::from(&f.loc), f.at))
            .map(|f| to(f))
            .collect()
    }

    pub fn walk_decorate(&mut self, start_path: &Path) {
        self.set_root(&dunce::canonicalize(start_path).unwrap());
        self.resources = Vec::<Res>::new();
        self.build(start_path, 0);
        self.resources.sort();
    }

    fn build(&mut self, src: &'a Path, lvl: usize) {
        let source = dunce::canonicalize(src).unwrap();

        if source.is_dir() {
            for entry in std::fs::read_dir(&source).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1);
                }

                self.resources.push(Res {
                    loc: path.to_path_buf(),
                    at: lvl,
                });
            }
        } else {
            self.resources.push(Res {
                loc: source,
                at: lvl,
            });
        }
    }
}

pub fn cp(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let mut source = PathBuf::from(args.shell_manager.path());
    let mut destination = PathBuf::from(args.shell_manager.path());
    let name_span = args.call_info.name_span;
    let args = args.evaluate_once(registry)?;

    match args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
        .as_string()?
        .as_str()
    {
        file => {
            source.push(file);
        }
    }

    match args
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
            args.nth(0).unwrap().span(),
        ));
    }

    let sources: Vec<_> = sources.unwrap().collect();

    if sources.len() == 1 {
        if let Ok(entry) = &sources[0] {
            if entry.is_dir() && !args.has("recursive") {
                return Err(ShellError::labeled_error(
                    "is a directory (not copied). Try using \"--recursive\".",
                    "is a directory (not copied). Try using \"--recursive\".",
                    args.nth(0).unwrap().span(),
                ));
            }

            let mut sources: FileStructure = FileStructure::new();

            sources.walk_decorate(&entry);

            if entry.is_file() {
                let strategy = |(source_file, _depth_level)| {
                    if destination.exists() {
                        let mut new_dst = dunce::canonicalize(destination.clone()).unwrap();
                        new_dst.push(entry.file_name().unwrap());
                        (source_file, new_dst)
                    } else {
                        (source_file, destination.clone())
                    }
                };

                for (ref src, ref dst) in sources.paths_applying_with(strategy) {
                    if src.is_file() {
                        match std::fs::copy(src, dst) {
                            Err(e) => {
                                return Err(ShellError::labeled_error(
                                    e.to_string(),
                                    e.to_string(),
                                    name_span,
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
                                name_span,
                            ));
                        }
                        Ok(o) => o,
                    };

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
                                            e.to_string(),
                                            e.to_string(),
                                            name_span,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }

                        if src.is_file() {
                            match std::fs::copy(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        e.to_string(),
                                        e.to_string(),
                                        name_span,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }
                } else {
                    destination.push(entry.file_name().unwrap());

                    match std::fs::create_dir_all(&destination) {
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                e.to_string(),
                                e.to_string(),
                                name_span,
                            ));
                        }
                        Ok(o) => o,
                    };

                    let strategy = |(source_file, depth_level)| {
                        let mut new_dst = dunce::canonicalize(&destination).unwrap();
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
                                            e.to_string(),
                                            e.to_string(),
                                            name_span,
                                        ));
                                    }
                                    Ok(o) => o,
                                };
                            }
                        }

                        if src.is_file() {
                            match std::fs::copy(src, dst) {
                                Err(e) => {
                                    return Err(ShellError::labeled_error(
                                        e.to_string(),
                                        e.to_string(),
                                        name_span,
                                    ));
                                }
                                Ok(o) => o,
                            };
                        }
                    }
                }
            }
        }
    } else {
        if destination.exists() {
            if !sources.iter().all(|x| (x.as_ref().unwrap()).is_file()) && !args.has("recursive") {
                return Err(ShellError::labeled_error(
                    "Copy aborted (directories found). Try using \"--recursive\".",
                    "Copy aborted (directories found). Try using \"--recursive\".",
                    args.nth(0).unwrap().span(),
                ));
            }

            for entry in sources {
                if let Ok(entry) = entry {
                    let mut to = PathBuf::from(&destination);
                    to.push(&entry.file_name().unwrap());

                    match std::fs::copy(&entry, &to) {
                        Err(e) => {
                            return Err(ShellError::labeled_error(
                                e.to_string(),
                                e.to_string(),
                                name_span,
                            ));
                        }
                        Ok(o) => o,
                    };
                }
            }
        } else {
            return Err(ShellError::labeled_error(
                format!(
                    "Copy aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                format!(
                    "Copy aborted. (Does {:?} exist?)",
                    &destination.file_name().unwrap()
                ),
                args.nth(1).unwrap().span(),
            ));
        }
    }

    Ok(OutputStream::empty())
}

#[cfg(test)]
mod tests {

    use super::{FileStructure, Res};
    use std::path::PathBuf;

    fn fixtures() -> PathBuf {
        let mut sdx = PathBuf::new();
        sdx.push("tests");
        sdx.push("fixtures");
        sdx.push("formats");
        dunce::canonicalize(sdx).unwrap()
    }

    #[test]
    fn prepares_and_decorates_source_files_for_copying() {
        let mut res = FileStructure::new();
        res.walk_decorate(fixtures().as_path());

        assert_eq!(
            res.resources,
            vec![
                Res {
                    loc: fixtures().join("appveyor.yml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("caco3_plastics.csv"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("cargo_sample.toml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("jonathan.xml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.ini"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sgml_description.json"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("utf16.ini"),
                    at: 0
                }
            ]
        );
    }
}
