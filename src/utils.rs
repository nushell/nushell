use crate::data::meta::Tagged;
use crate::data::Value;
use crate::errors::ShellError;
use crate::{PathMember, RawPathMember};
use std::fmt;
use std::ops::Div;
use std::path::{Component, Path, PathBuf};

pub fn did_you_mean(obj_source: &Value, field_tried: &PathMember) -> Option<Vec<(usize, String)>> {
    let field_tried = match &field_tried.item {
        RawPathMember::String(string) => string.clone(),
        RawPathMember::Int(int) => format!("{}", int),
    };

    let possibilities = obj_source.data_descriptors();

    let mut possible_matches: Vec<_> = possibilities
        .into_iter()
        .map(|x| {
            let word = x.clone();
            let distance = natural::distance::levenshtein_distance(&word, &field_tried);

            (distance, word)
        })
        .collect();

    if possible_matches.len() > 0 {
        possible_matches.sort();
        Some(possible_matches)
    } else {
        None
    }
}

pub struct AbsoluteFile {
    inner: PathBuf,
}

impl AbsoluteFile {
    pub fn new(path: impl AsRef<Path>) -> AbsoluteFile {
        let path = path.as_ref();

        if !path.is_absolute() {
            panic!(
                "AbsoluteFile::new must take an absolute path :: {}",
                path.display()
            )
        } else if path.is_dir() {
            // At the moment, this is not an invariant, but rather a way to catch bugs
            // in tests.
            panic!(
                "AbsoluteFile::new must not take a directory :: {}",
                path.display()
            )
        } else {
            AbsoluteFile {
                inner: path.to_path_buf(),
            }
        }
    }

    pub fn dir(&self) -> AbsolutePath {
        AbsolutePath::new(self.inner.parent().unwrap())
    }
}

impl From<AbsoluteFile> for PathBuf {
    fn from(file: AbsoluteFile) -> Self {
        file.inner
    }
}

impl fmt::Display for AbsoluteFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

pub struct AbsolutePath {
    inner: PathBuf,
}

impl AbsolutePath {
    pub fn new(path: impl AsRef<Path>) -> AbsolutePath {
        let path = path.as_ref();

        if path.is_absolute() {
            AbsolutePath {
                inner: path.to_path_buf(),
            }
        } else {
            panic!("AbsolutePath::new must take an absolute path")
        }
    }
}

impl fmt::Display for AbsolutePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

impl Div<&str> for &AbsolutePath {
    type Output = AbsolutePath;

    fn div(self, rhs: &str) -> Self::Output {
        let parts = rhs.split("/");
        let mut result = self.inner.clone();

        for part in parts {
            result = result.join(part);
        }

        AbsolutePath::new(result)
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.inner.as_path()
    }
}

pub struct RelativePath {
    inner: PathBuf,
}

impl RelativePath {
    pub fn new(path: impl Into<PathBuf>) -> RelativePath {
        let path = path.into();

        if path.is_relative() {
            RelativePath { inner: path }
        } else {
            panic!("RelativePath::new must take a relative path")
        }
    }
}

impl<T: AsRef<str>> Div<T> for &RelativePath {
    type Output = RelativePath;

    fn div(self, rhs: T) -> Self::Output {
        let parts = rhs.as_ref().split("/");
        let mut result = self.inner.clone();

        for part in parts {
            result = result.join(part);
        }

        RelativePath::new(result)
    }
}

impl fmt::Display for RelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

pub enum TaggedValueIter<'a> {
    Empty,
    List(indexmap::map::Iter<'a, String, Tagged<Value>>),
}

impl<'a> Iterator for TaggedValueIter<'a> {
    type Item = (&'a String, &'a Tagged<Value>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TaggedValueIter::Empty => None,
            TaggedValueIter::List(iter) => iter.next(),
        }
    }
}

impl Tagged<Value> {
    fn is_dir(&self) -> bool {
        match self.item() {
            Value::Row(_) | Value::Table(_) => true,
            _ => false,
        }
    }

    fn entries(&self) -> TaggedValueIter<'_> {
        match self.item() {
            Value::Row(o) => {
                let iter = o.entries.iter();
                TaggedValueIter::List(iter)
            }
            _ => TaggedValueIter::Empty,
        }
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValueResource {
    pub at: usize,
    pub loc: PathBuf,
}

impl ValueResource {}

pub struct ValueStructure {
    pub resources: Vec<ValueResource>,
}

impl ValueStructure {
    pub fn new() -> ValueStructure {
        ValueStructure {
            resources: Vec::<ValueResource>::new(),
        }
    }

    pub fn exists(&self, path: &Path) -> bool {
        if path == Path::new("/") {
            return true;
        }

        let path = if path.starts_with("/") {
            match path.strip_prefix("/") {
                Ok(p) => p,
                Err(_) => path,
            }
        } else {
            path
        };

        let comps: Vec<_> = path.components().map(Component::as_os_str).collect();

        let mut is_there = true;

        for (at, fragment) in comps.iter().enumerate() {
            is_there = is_there
                && self
                    .resources
                    .iter()
                    .any(|resource| at == resource.at && *fragment == resource.loc.as_os_str());
        }

        is_there
    }

    pub fn walk_decorate(&mut self, start: &Tagged<Value>) -> Result<(), ShellError> {
        self.resources = Vec::<ValueResource>::new();
        self.build(start, 0)?;
        self.resources.sort();

        Ok(())
    }

    fn build(&mut self, src: &Tagged<Value>, lvl: usize) -> Result<(), ShellError> {
        for entry in src.entries() {
            let value = entry.1;
            let path = entry.0;

            self.resources.push(ValueResource {
                at: lvl,
                loc: PathBuf::from(path),
            });

            if value.is_dir() {
                self.build(value, lvl + 1)?;
            }
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

pub struct FileStructure {
    pub resources: Vec<Res>,
}

impl FileStructure {
    pub fn new() -> FileStructure {
        FileStructure {
            resources: Vec::<Res>::new(),
        }
    }

    pub fn contains_more_than_one_file(&self) -> bool {
        self.resources.len() > 1
    }

    pub fn contains_files(&self) -> bool {
        self.resources.len() > 0
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
        let source = dunce::canonicalize(src)?;

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

#[cfg(test)]
mod tests {
    use super::{FileStructure, Res, ValueResource, ValueStructure};
    use crate::data::meta::{Tag, Tagged};
    use crate::data::{TaggedDictBuilder, Value};
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    fn fixtures() -> PathBuf {
        let mut sdx = PathBuf::new();
        sdx.push("tests");
        sdx.push("fixtures");
        sdx.push("formats");

        match dunce::canonicalize(sdx) {
            Ok(path) => path,
            Err(_) => panic!("Wrong path."),
        }
    }

    fn structured_sample_record(key: &str, value: &str) -> Tagged<Value> {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert(key.clone(), Value::string(value));
        record.into_tagged_value()
    }

    fn sample_nushell_source_code() -> Tagged<Value> {
        /*
            src
             commands
              plugins => "sys.rs"
             tests
              helpers => "mod.rs"
        */

        let mut src = TaggedDictBuilder::new(Tag::unknown());
        let mut record = TaggedDictBuilder::new(Tag::unknown());

        record.insert_tagged("commands", structured_sample_record("plugins", "sys.rs"));
        record.insert_tagged("tests", structured_sample_record("helpers", "mod.rs"));
        src.insert_tagged("src", record.into_tagged_value());

        src.into_tagged_value()
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
        let mut res = FileStructure::new();

        res.walk_decorate(&fixtures())
            .expect("Can not decorate files traversal.");

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
                    loc: fixtures().join("caco3_plastics.tsv"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("cargo_sample.toml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("fileA.txt"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("jonathan.xml"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.bson"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.db"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.ini"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample.url"),
                    at: 0
                },
                Res {
                    loc: fixtures().join("sample_data.xlsx"),
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
