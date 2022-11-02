use filesize::file_real_size_fast;
use nu_glob::Pattern;
use nu_protocol::{ShellError, Span, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DirBuilder {
    pub tag: Span,
    pub min: Option<u64>,
    pub deref: bool,
    pub exclude: Option<Pattern>,
    pub all: bool,
}

impl DirBuilder {
    pub fn new(
        tag: Span,
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

#[derive(Debug, Clone)]
pub struct DirInfo {
    dirs: Vec<DirInfo>,
    files: Vec<FileInfo>,
    errors: Vec<ShellError>,
    size: u64,
    blocks: u64,
    path: PathBuf,
    tag: Span,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    path: PathBuf,
    size: u64,
    blocks: Option<u64>,
    tag: Span,
}

impl FileInfo {
    pub fn new(path: impl Into<PathBuf>, deref: bool, tag: Span) -> Result<Self, ShellError> {
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
        ctrl_c: Option<Arc<AtomicBool>>,
    ) -> Self {
        let path = path.into();

        let mut s = Self {
            dirs: Vec::new(),
            errors: Vec::new(),
            files: Vec::new(),
            size: 0,
            blocks: 0,
            tag: params.tag,
            path,
        };

        match std::fs::metadata(&s.path) {
            Ok(d) => {
                s.size = d.len(); // dir entry size
                s.blocks = file_real_size_fast(&s.path, &d).ok().unwrap_or(0);
            }
            Err(e) => s = s.add_error(e.into()),
        };

        match std::fs::read_dir(&s.path) {
            Ok(d) => {
                for f in d {
                    match ctrl_c {
                        Some(ref cc) => {
                            if cc.load(Ordering::SeqCst) {
                                break;
                            }
                        }
                        None => continue,
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
        ctrl_c: Option<Arc<AtomicBool>>,
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
            match FileInfo::new(f, params.deref, self.tag) {
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
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("path".into());
        vals.push(Value::string(d.path.display().to_string(), d.tag));

        cols.push("apparent".into());
        vals.push(Value::Filesize {
            val: d.size as i64,
            span: d.tag,
        });

        cols.push("physical".into());
        vals.push(Value::Filesize {
            val: d.blocks as i64,
            span: d.tag,
        });

        cols.push("directories".into());
        vals.push(value_from_vec(d.dirs, &d.tag));

        cols.push("files".into());
        vals.push(value_from_vec(d.files, &d.tag));

        // if !d.errors.is_empty() {
        //     let v = d
        //         .errors
        //         .into_iter()
        //         .map(move |e| Value::Error { error: e })
        //         .collect::<Vec<Value>>();

        //     cols.push("errors".into());
        //     vals.push(Value::List {
        //         vals: v,
        //         span: d.tag,
        //     })
        // }

        Value::Record {
            cols,
            vals,
            span: d.tag,
        }
    }
}

impl From<FileInfo> for Value {
    fn from(f: FileInfo) -> Self {
        let mut cols = vec![];
        let mut vals = vec![];

        cols.push("path".into());
        vals.push(Value::string(f.path.display().to_string(), f.tag));

        cols.push("apparent".into());
        vals.push(Value::Filesize {
            val: f.size as i64,
            span: f.tag,
        });

        cols.push("physical".into());
        vals.push(Value::Filesize {
            val: match f.blocks {
                Some(b) => b as i64,
                None => 0i64,
            },
            span: f.tag,
        });

        cols.push("directories".into());
        vals.push(Value::nothing(Span::test_data()));

        cols.push("files".into());
        vals.push(Value::nothing(Span::test_data()));

        // cols.push("errors".into());
        // vals.push(Value::nothing(Span::test_data()));

        Value::Record {
            cols,
            vals,
            span: f.tag,
        }
    }
}

fn value_from_vec<V>(vec: Vec<V>, tag: &Span) -> Value
where
    V: Into<Value>,
{
    if vec.is_empty() {
        Value::nothing(*tag)
    } else {
        let values = vec.into_iter().map(Into::into).collect::<Vec<Value>>();
        Value::List {
            vals: values,
            span: *tag,
        }
    }
}
