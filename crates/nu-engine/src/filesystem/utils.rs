use crate::filesystem::path::canonicalize;
use nu_errors::ShellError;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct FileStructure {
    pub resources: Vec<Res>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure {
            resources: Vec::<Res>::new(),
        }
    }

    #[allow(dead_code)]
    pub fn contains_more_than_one_file(&self) -> bool {
        self.resources.len() > 1
    }

    #[allow(dead_code)]
    pub fn contains_files(&self) -> bool {
        !self.resources.is_empty()
    }

    pub fn paths_applying_with<F>(
        &mut self,
        to: F,
    ) -> Result<Vec<(PathBuf, PathBuf)>, Box<dyn std::error::Error>>
    where
        F: Fn((PathBuf, usize)) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>>,
    {
        self.resources
            .iter()
            .map(|f| (PathBuf::from(&f.loc), f.at))
            .map(|f| to(f))
            .collect()
    }

    pub fn walk_decorate(&mut self, start_path: &Path) -> Result<(), ShellError> {
        self.resources = Vec::<Res>::new();
        self.build(start_path, 0)?;
        self.resources.sort();

        Ok(())
    }

    fn build(&mut self, src: &Path, lvl: usize) -> Result<(), ShellError> {
        let source = canonicalize(std::env::current_dir()?, src)?;

        if source.is_dir() {
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    self.build(&path, lvl + 1)?;
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

        Ok(())
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Res {
    pub at: usize,
    pub loc: PathBuf,
}

impl Res {}

#[cfg(test)]
mod tests {
    use super::{FileStructure, Res};
    use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value, ValueResource, ValueStructure};
    use nu_source::Tag;
    use nu_test_support::{fs::Stub::EmptyFile, playground::Playground};
    use std::path::PathBuf;

    fn structured_sample_record(key: &str, value: &str) -> Value {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert_untagged(key, UntaggedValue::string(value));
        record.into_value()
    }

    fn sample_nushell_source_code() -> Value {
        /*
            src
             commands
              plugins => "sys.rs"
             tests
              helpers => "mod.rs"
        */

        let mut src = TaggedDictBuilder::new(Tag::unknown());
        let mut record = TaggedDictBuilder::new(Tag::unknown());

        record.insert_value("commands", structured_sample_record("plugins", "sys.rs"));
        record.insert_value("tests", structured_sample_record("helpers", "mod.rs"));
        src.insert_value("src", record.into_value());

        src.into_value()
    }

    #[test]
    fn prepares_and_decorates_value_filesystemlike_sources() {
        let mut res = ValueStructure::new();

        res.walk_decorate(&sample_nushell_source_code())
            .expect("Can not decorate values traversal.");

        assert_eq!(
            res.resources,
            vec![
                ValueResource {
                    loc: PathBuf::from("src"),
                    at: 0,
                },
                ValueResource {
                    loc: PathBuf::from("commands"),
                    at: 1,
                },
                ValueResource {
                    loc: PathBuf::from("tests"),
                    at: 1,
                },
                ValueResource {
                    loc: PathBuf::from("helpers"),
                    at: 2,
                },
                ValueResource {
                    loc: PathBuf::from("plugins"),
                    at: 2,
                },
            ]
        );
    }

    #[test]
    fn recognizes_if_path_exists_in_value_filesystemlike_sources() {
        let mut res = ValueStructure::new();

        res.walk_decorate(&sample_nushell_source_code())
            .expect("Can not decorate values traversal.");

        assert!(res.exists(&PathBuf::from("/")));

        assert!(res.exists(&PathBuf::from("src/commands/plugins")));
        assert!(res.exists(&PathBuf::from("src/commands")));
        assert!(res.exists(&PathBuf::from("src/tests")));
        assert!(res.exists(&PathBuf::from("src/tests/helpers")));
        assert!(res.exists(&PathBuf::from("src")));

        assert!(res.exists(&PathBuf::from("/src/commands/plugins")));
        assert!(res.exists(&PathBuf::from("/src/commands")));
        assert!(res.exists(&PathBuf::from("/src/tests")));
        assert!(res.exists(&PathBuf::from("/src/tests/helpers")));
        assert!(res.exists(&PathBuf::from("/src")));

        assert!(!res.exists(&PathBuf::from("/not_valid")));
        assert!(!res.exists(&PathBuf::from("/src/not_valid")));
    }

    #[test]
    fn prepares_and_decorates_filesystem_source_files() {
        Playground::setup("file_structure_test", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("sample.ini"),
                EmptyFile("sample.eml"),
                EmptyFile("cargo_sample.toml"),
            ]);

            let mut res = FileStructure::new();

            res.walk_decorate(&dirs.test())
                .expect("Can not decorate files traversal.");

            assert_eq!(
                res.resources,
                vec![
                    Res {
                        loc: dirs.test().join("cargo_sample.toml"),
                        at: 0
                    },
                    Res {
                        loc: dirs.test().join("sample.eml"),
                        at: 0
                    },
                    Res {
                        loc: dirs.test().join("sample.ini"),
                        at: 0
                    }
                ]
            );
        })
    }
}
