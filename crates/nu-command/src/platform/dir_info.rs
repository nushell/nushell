use filesize::file_real_size_fast;
use nu_glob::Pattern;
use nu_protocol::{ShellError, Signals, Span, Value, record, shell_error::io::IoError};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DirBuilder {
    pub tag: Span,
    pub min: Option<u64>,
    pub deref: bool,
    pub exclude: Option<Pattern>,
    pub long: bool,
}

impl DirBuilder {
    pub fn new(
        tag: Span,
        min: Option<u64>,
        deref: bool,
        exclude: Option<Pattern>,
        long: bool,
    ) -> DirBuilder {
        DirBuilder {
            tag,
            min,
            deref,
            exclude,
            long,
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
    long: bool,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    path: PathBuf,
    size: u64,
    blocks: Option<u64>,
    tag: Span,
    long: bool,
}

impl FileInfo {
    pub fn new(
        path: impl Into<PathBuf>,
        deref: bool,
        tag: Span,
        long: bool,
    ) -> Result<Self, ShellError> {
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
                    long,
                })
            }
            Err(e) => Err(IoError::new(e, tag, path).into()),
        }
    }
}

impl DirInfo {
    pub fn new(
        path: impl Into<PathBuf>,
        params: &DirBuilder,
        depth: Option<u64>,
        span: Span,
        signals: &Signals,
    ) -> Result<Self, ShellError> {
        let path = path.into();
        let from_io_error = IoError::factory(span, path.as_path());

        let mut s = Self {
            dirs: Vec::new(),
            errors: Vec::new(),
            files: Vec::new(),
            size: 0,
            blocks: 0,
            tag: params.tag,
            path: path.clone(),
            long: params.long,
        };

        match std::fs::metadata(&s.path) {
            Ok(d) => {
                s.size = d.len(); // dir entry size
                s.blocks = file_real_size_fast(&s.path, &d).ok().unwrap_or(0);
            }
            Err(e) => s = s.add_error(from_io_error(e).into()),
        };

        match std::fs::read_dir(&s.path) {
            Ok(d) => {
                for f in d {
                    signals.check(&span)?;

                    match f {
                        Ok(i) => match i.file_type() {
                            Ok(t) if t.is_dir() => {
                                s = s.add_dir(i.path(), depth, params, span, signals)?
                            }
                            Ok(_t) => s = s.add_file(i.path(), params),
                            Err(e) => s = s.add_error(from_io_error(e).into()),
                        },
                        Err(e) => s = s.add_error(from_io_error(e).into()),
                    }
                }
            }
            Err(e) => s = s.add_error(from_io_error(e).into()),
        }
        Ok(s)
    }

    fn add_dir(
        mut self,
        path: impl Into<PathBuf>,
        mut depth: Option<u64>,
        params: &DirBuilder,
        span: Span,
        signals: &Signals,
    ) -> Result<Self, ShellError> {
        if let Some(current) = depth {
            if let Some(new) = current.checked_sub(1) {
                depth = Some(new);
            } else {
                return Ok(self);
            }
        }

        let d = DirInfo::new(path, params, depth, span, signals)?;
        self.size += d.size;
        self.blocks += d.blocks;
        self.dirs.push(d);
        Ok(self)
    }

    fn add_file(mut self, f: impl Into<PathBuf>, params: &DirBuilder) -> Self {
        let f = f.into();
        let include = params.exclude.as_ref().is_none_or(|x| !x.matches_path(&f));
        if include {
            match FileInfo::new(f, params.deref, self.tag, self.long) {
                Ok(file) => {
                    let inc = params.min.is_none_or(|s| file.size >= s);
                    if inc {
                        self.size += file.size;
                        self.blocks += file.blocks.unwrap_or(0);
                        if params.long {
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

        if d.long {
            Value::record(
                record! {
                    "path" => Value::string(d.path.display().to_string(), d.tag),
                    "apparent" => Value::filesize(d.size as i64, d.tag),
                    "physical" => Value::filesize(d.blocks as i64, d.tag),
                    "directories" => value_from_vec(d.dirs, d.tag),
                    "files" => value_from_vec(d.files, d.tag)
                },
                d.tag,
            )
        } else {
            Value::record(
                record! {
                    "path" => Value::string(d.path.display().to_string(), d.tag),
                    "apparent" => Value::filesize(d.size as i64, d.tag),
                    "physical" => Value::filesize(d.blocks as i64, d.tag),
                },
                d.tag,
            )
        }
    }
}

impl From<FileInfo> for Value {
    fn from(f: FileInfo) -> Self {
        // cols.push("errors".into());
        // vals.push(Value::nothing(Span::unknown()));

        if f.long {
            Value::record(
                record! {
                    "path" => Value::string(f.path.display().to_string(), f.tag),
                    "apparent" => Value::filesize(f.size as i64, f.tag),
                    "physical" => Value::filesize(f.blocks.unwrap_or(0) as i64, f.tag),
                    "directories" => Value::nothing(Span::unknown()),
                    "files" => Value::nothing(Span::unknown()),
                },
                f.tag,
            )
        } else {
            Value::record(
                record! {
                    "path" => Value::string(f.path.display().to_string(), f.tag),
                    "apparent" => Value::filesize(f.size as i64, f.tag),
                    "physical" => Value::filesize(f.blocks.unwrap_or(0) as i64, f.tag),
                },
                f.tag,
            )
        }
    }
}

fn value_from_vec<V>(vec: Vec<V>, tag: Span) -> Value
where
    V: Into<Value>,
{
    if vec.is_empty() {
        Value::nothing(tag)
    } else {
        let values = vec.into_iter().map(Into::into).collect::<Vec<Value>>();
        Value::list(values, tag)
    }
}
