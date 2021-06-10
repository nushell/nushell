use filesize::file_real_size_fast;
use glob::Pattern;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{UntaggedValue, Value};
use nu_source::Tag;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DirBuilder {
    pub tag: Tag,
    pub min: Option<u64>,
    pub deref: bool,
    pub exclude: Option<Pattern>,
    pub all: bool,
}

impl DirBuilder {
    pub fn new(
        tag: Tag,
        min: Option<u64>,
        deref: bool,
        exclude: Option<Pattern>,
        all: bool,
    ) -> DirBuilder {
        DirBuilder {
            tag,
            min,
            deref,
            exclude,
            all,
        }
    }
}

pub struct DirInfo {
    dirs: Vec<DirInfo>,
    files: Vec<FileInfo>,
    errors: Vec<ShellError>,
    size: u64,
    blocks: u64,
    path: PathBuf,
    tag: Tag,
}

pub struct FileInfo {
    path: PathBuf,
    size: u64,
    blocks: Option<u64>,
    tag: Tag,
}

impl FileInfo {
    pub fn new(path: impl Into<PathBuf>, deref: bool, tag: Tag) -> Result<Self, ShellError> {
        let path = path.into();
        let m = if deref {
            std::fs::metadata(&path)
        } else {
            std::fs::symlink_metadata(&path)
        };

        match m {
            Ok(d) => {
                let block_size = file_real_size_fast(&path, &d).ok();

                Ok(FileInfo {
                    path,
                    blocks: block_size,
                    size: d.len(),
                    tag,
                })
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl DirInfo {
    pub fn new(
        path: impl Into<PathBuf>,
        params: &DirBuilder,
        depth: Option<u64>,
        ctrl_c: Arc<AtomicBool>,
    ) -> Self {
        let path = path.into();

        let mut s = Self {
            dirs: Vec::new(),
            errors: Vec::new(),
            files: Vec::new(),
            size: 0,
            blocks: 0,
            tag: params.tag.clone(),
            path,
        };

        match std::fs::read_dir(&s.path) {
            Ok(d) => {
                for f in d {
                    if ctrl_c.load(Ordering::SeqCst) {
                        break;
                    }

                    match f {
                        Ok(i) => match i.file_type() {
                            Ok(t) if t.is_dir() => {
                                s = s.add_dir(i.path(), depth, params, ctrl_c.clone())
                            }
                            Ok(_t) => s = s.add_file(i.path(), params),
                            Err(e) => s = s.add_error(e.into()),
                        },
                        Err(e) => s = s.add_error(e.into()),
                    }
                }
            }
            Err(e) => s = s.add_error(e.into()),
        }
        s
    }

    fn add_dir(
        mut self,
        path: impl Into<PathBuf>,
        mut depth: Option<u64>,
        params: &DirBuilder,
        ctrl_c: Arc<AtomicBool>,
    ) -> Self {
        if let Some(current) = depth {
            if let Some(new) = current.checked_sub(1) {
                depth = Some(new);
            } else {
                return self;
            }
        }

        let d = DirInfo::new(path, params, depth, ctrl_c);
        self.size += d.size;
        self.blocks += d.blocks;
        self.dirs.push(d);
        self
    }

    fn add_file(mut self, f: impl Into<PathBuf>, params: &DirBuilder) -> Self {
        let f = f.into();
        let include = params
            .exclude
            .as_ref()
            .map_or(true, |x| !x.matches_path(&f));
        if include {
            match FileInfo::new(f, params.deref, self.tag.clone()) {
                Ok(file) => {
                    let inc = params.min.map_or(true, |s| file.size >= s);
                    if inc {
                        self.size += file.size;
                        self.blocks += file.blocks.unwrap_or(0);
                        if params.all {
                            self.files.push(file);
                        }
                    }
                }
                Err(e) => self = self.add_error(e),
            }
        }
        self
    }

    fn add_error(mut self, e: ShellError) -> Self {
        self.errors.push(e);
        self
    }

    pub fn get_size(&self) -> u64 {
        self.size
    }
}

impl From<DirInfo> for Value {
    fn from(d: DirInfo) -> Self {
        let mut r: IndexMap<String, Value> = IndexMap::new();

        r.insert(
            "path".to_string(),
            UntaggedValue::filepath(d.path).into_value(&d.tag),
        );

        r.insert(
            "apparent".to_string(),
            UntaggedValue::filesize(d.size).into_value(&d.tag),
        );

        r.insert(
            "physical".to_string(),
            UntaggedValue::filesize(d.blocks).into_value(&d.tag),
        );

        r.insert("directories".to_string(), value_from_vec(d.dirs, &d.tag));

        r.insert("files".to_string(), value_from_vec(d.files, &d.tag));

        if !d.errors.is_empty() {
            let v = UntaggedValue::Table(
                d.errors
                    .into_iter()
                    .map(move |e| UntaggedValue::Error(e).into_untagged_value())
                    .collect::<Vec<Value>>(),
            )
            .into_value(&d.tag);

            r.insert("errors".to_string(), v);
        }

        Value {
            value: UntaggedValue::row(r),
            tag: d.tag,
        }
    }
}

impl From<FileInfo> for Value {
    fn from(f: FileInfo) -> Self {
        let mut r: IndexMap<String, Value> = IndexMap::new();

        r.insert(
            "path".to_string(),
            UntaggedValue::filepath(f.path).into_value(&f.tag),
        );

        r.insert(
            "apparent".to_string(),
            UntaggedValue::filesize(f.size).into_value(&f.tag),
        );

        let b = f
            .blocks
            .map(UntaggedValue::filesize)
            .unwrap_or_else(UntaggedValue::nothing)
            .into_value(&f.tag);

        r.insert("physical".to_string(), b);

        r.insert(
            "directories".to_string(),
            UntaggedValue::nothing().into_value(&f.tag),
        );

        r.insert(
            "files".to_string(),
            UntaggedValue::nothing().into_value(&f.tag),
        );

        UntaggedValue::row(r).into_value(&f.tag)
    }
}

fn value_from_vec<V>(vec: Vec<V>, tag: &Tag) -> Value
where
    V: Into<Value>,
{
    if vec.is_empty() {
        UntaggedValue::nothing()
    } else {
        let values = vec.into_iter().map(Into::into).collect::<Vec<Value>>();
        UntaggedValue::Table(values)
    }
    .into_value(tag)
}
