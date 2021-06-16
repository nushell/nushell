use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{UntaggedValue, Value};
use std::cell::RefCell;
use std::ffi::{OsStr, OsString};

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub config: Option<OsString>,
    pub stdin: bool,
    pub scripts: Vec<NuScript>,
    pub save_history: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CliOptions {
    pub fn new() -> Self {
        Self {
            config: None,
            stdin: false,
            scripts: vec![],
            save_history: true,
        }
    }
}

#[derive(Debug)]
pub struct Options {
    inner: RefCell<IndexMap<String, Value>>,
}

impl Options {
    pub fn default() -> Self {
        Self {
            inner: RefCell::new(IndexMap::default()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.inner.borrow().get(key).map(Clone::clone)
    }

    pub fn put(&self, key: &str, value: Value) {
        self.inner.borrow_mut().insert(key.into(), value);
    }

    pub fn shift(&self) {
        if let Some(Value {
            value: UntaggedValue::Table(ref mut args),
            ..
        }) = self.inner.borrow_mut().get_mut("args")
        {
            args.remove(0);
        }
    }

    pub fn swap(&self, other: &Options) {
        self.inner.swap(&other.inner);
    }
}

#[derive(Debug, Clone)]
pub struct NuScript {
    pub filepath: Option<OsString>,
    pub contents: String,
}

impl NuScript {
    pub fn code(content: &str) -> Result<Self, ShellError> {
        Ok(Self {
            filepath: None,
            contents: content.to_string(),
        })
    }

    pub fn get_code(&self) -> &str {
        &self.contents
    }

    pub fn source_file(path: &OsStr) -> Result<Self, ShellError> {
        use std::fs::File;
        use std::io::Read;

        let path = path.to_os_string();
        let mut file = File::open(&path)?;
        let mut buffer = String::new();

        file.read_to_string(&mut buffer)?;

        Ok(Self {
            filepath: Some(path),
            contents: buffer,
        })
    }
}
