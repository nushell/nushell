use crate::{UntaggedValue, Value};
use nu_errors::ShellError;
use std::path::{Component, Path, PathBuf};

fn is_value_tagged_dir(value: &Value) -> bool {
    matches!(
        &value.value,
        UntaggedValue::Row(_) | UntaggedValue::Table(_)
    )
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValueResource {
    pub at: usize,
    pub loc: PathBuf,
}

impl ValueResource {}

#[derive(Default)]
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
            path.strip_prefix("/").unwrap_or(path)
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

    pub fn walk_decorate(&mut self, start: &Value) -> Result<(), ShellError> {
        self.resources = Vec::<ValueResource>::new();
        self.build(start, 0)?;
        self.resources.sort();

        Ok(())
    }

    fn build(&mut self, src: &Value, lvl: usize) -> Result<(), ShellError> {
        for entry in src.row_entries() {
            let value = entry.1;
            let path = entry.0;

            self.resources.push(ValueResource {
                at: lvl,
                loc: PathBuf::from(path),
            });

            if is_value_tagged_dir(value) {
                self.build(value, lvl + 1)?;
            }
        }

        Ok(())
    }
}
