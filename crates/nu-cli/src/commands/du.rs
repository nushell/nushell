extern crate filesize;

use crate::commands::command::RunnablePerItemContext;
use crate::prelude::*;
use filesize::file_real_size_fast;
use glob::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{CallInfo, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

const NAME: &str = "du";
const GLOB_PARAMS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

pub struct Du;

#[derive(Deserialize, Clone)]
pub struct DuArgs {
    path: Option<Tagged<PathBuf>>,
    all: bool,
    deref: bool,
    exclude: Option<Tagged<String>>,
    #[serde(rename = "max-depth")]
    max_depth: Option<Tagged<u64>>,
    #[serde(rename = "min-size")]
    min_size: Option<Tagged<u64>>,
}

impl PerItemCommand for Du {
    fn name(&self) -> &str {
        NAME
    }

    fn signature(&self) -> Signature {
        Signature::build(NAME)
            .optional("path", SyntaxShape::Pattern, "starting directory")
            .switch(
                "all",
                "Output File sizes as well as directory sizes",
                Some('a'),
            )
            .switch(
                "deref",
                "Dereference symlinks to their targets for size",
                Some('r'),
            )
            .named(
                "exclude",
                SyntaxShape::Pattern,
                "Exclude these file names",
                Some('x'),
            )
            .named(
                "max-depth",
                SyntaxShape::Int,
                "Directory recursion limit",
                Some('d'),
            )
            .named(
                "min-size",
                SyntaxShape::Int,
                "Exclude files below this size",
                Some('m'),
            )
    }

    fn usage(&self) -> &str {
        "Find disk usage sizes of specified items"
    }

    fn run(
        &self,
        call_info: &CallInfo,
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Value,
    ) -> Result<OutputStream, ShellError> {
        call_info
            .process(&raw_args.shell_manager, raw_args.ctrl_c.clone(), du)?
            .run()
    }
}

fn du(args: DuArgs, ctx: &RunnablePerItemContext) -> Result<OutputStream, ShellError> {
    let tag = ctx.name.clone();

    let exclude = args
        .exclude
        .clone()
        .map_or(Ok(None), move |x| match Pattern::new(&x.item) {
            Ok(p) => Ok(Some(p)),
            Err(e) => Err(ShellError::labeled_error(
                e.msg,
                "Glob error",
                x.tag.clone(),
            )),
        })?;
    let path = args.path.clone();
    let filter_files = path.is_none();
    let paths = match path {
        Some(p) => match glob::glob_with(
            p.item.to_str().expect("Why isn't this encoded properly?"),
            GLOB_PARAMS,
        ) {
            Ok(g) => Ok(g),
            Err(e) => Err(ShellError::labeled_error(
                e.msg,
                "Glob error",
                p.tag.clone(),
            )),
        },
        None => match glob::glob_with("*", GLOB_PARAMS) {
            Ok(g) => Ok(g),
            Err(e) => Err(ShellError::labeled_error(e.msg, "Glob error", tag.clone())),
        },
    }?
    .filter(move |p| {
        if filter_files {
            match p {
                Ok(f) if f.is_dir() => true,
                Err(e) if e.path().is_dir() => true,
                _ => false,
            }
        } else {
            true
        }
    })
    .map(move |p| match p {
        Err(e) => Err(glob_err_into(e)),
        Ok(s) => Ok(s),
    });

    let ctrl_c = ctx.ctrl_c.clone();
    let all = args.all;
    let deref = args.deref;
    let max_depth = args.max_depth.map(|f| f.item);
    let min_size = args.min_size.map(|f| f.item);

    let stream = async_stream! {
        let params = DirBuilder {
            tag: tag.clone(),
            min: min_size,
            deref,
            ex: exclude,
            all,
        };
        for path in paths {
            if ctrl_c.load(Ordering::SeqCst) {
                break;
            }
            match path {
                Ok(p) => {
                    if p.is_dir() {
                        yield Ok(ReturnSuccess::Value(
                            DirInfo::new(p, &params, max_depth).into(),
                        ));
                    } else {
                        match FileInfo::new(p, deref, tag.clone()) {
                            Ok(f) => yield Ok(ReturnSuccess::Value(f.into())),
                            Err(e) => yield Err(e)
                        }
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };
    Ok(stream.to_output_stream())
}

struct DirBuilder {
    tag: Tag,
    min: Option<u64>,
    deref: bool,
    ex: Option<Pattern>,
    all: bool,
}

struct DirInfo {
    dirs: Vec<DirInfo>,
    files: Vec<FileInfo>,
    errors: Vec<ShellError>,
    size: u64,
    blocks: u64,
    path: PathBuf,
    tag: Tag,
}

struct FileInfo {
    path: PathBuf,
    size: u64,
    blocks: Option<u64>,
    tag: Tag,
}

impl FileInfo {
    fn new(path: impl Into<PathBuf>, deref: bool, tag: Tag) -> Result<Self, ShellError> {
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
    fn new(path: impl Into<PathBuf>, params: &DirBuilder, depth: Option<u64>) -> Self {
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
                    match f {
                        Ok(i) => match i.file_type() {
                            Ok(t) if t.is_dir() => {
                                s = s.add_dir(i.path(), depth, &params);
                            }
                            Ok(_t) => {
                                s = s.add_file(i.path(), &params);
                            }
                            Err(e) => s = s.add_error(ShellError::from(e)),
                        },
                        Err(e) => s = s.add_error(ShellError::from(e)),
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
    ) -> Self {
        if let Some(current) = depth {
            if let Some(new) = current.checked_sub(1) {
                depth = Some(new);
            } else {
                return self;
            }
        }

        let d = DirInfo::new(path, &params, depth);
        self.size += d.size;
        self.blocks += d.blocks;
        self.dirs.push(d);
        self
    }

    fn add_file(mut self, f: impl Into<PathBuf>, params: &DirBuilder) -> Self {
        let f = f.into();
        let ex = params.ex.as_ref().map_or(false, |x| x.matches_path(&f));
        if !ex {
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
}

fn glob_err_into(e: GlobError) -> ShellError {
    let e = e.into_error();
    ShellError::from(e)
}

impl From<DirInfo> for Value {
    fn from(d: DirInfo) -> Self {
        let mut r: IndexMap<String, Value> = IndexMap::new();
        r.insert(
            "path".to_string(),
            UntaggedValue::path(d.path).retag(d.tag.clone()),
        );
        r.insert(
            "apparent".to_string(),
            UntaggedValue::bytes(d.size).retag(d.tag.clone()),
        );
        r.insert(
            "physical".to_string(),
            UntaggedValue::bytes(d.blocks).retag(d.tag.clone()),
        );
        if !d.files.is_empty() {
            let v = Value {
                value: UntaggedValue::Table(
                    d.files
                        .into_iter()
                        .map(move |f| f.into())
                        .collect::<Vec<Value>>(),
                ),
                tag: d.tag.clone(),
            };
            r.insert("files".to_string(), v);
        }
        if !d.dirs.is_empty() {
            let v = Value {
                value: UntaggedValue::Table(
                    d.dirs
                        .into_iter()
                        .map(move |d| d.into())
                        .collect::<Vec<Value>>(),
                ),
                tag: d.tag.clone(),
            };
            r.insert("directories".to_string(), v);
        }
        if !d.errors.is_empty() {
            let v = Value {
                value: UntaggedValue::Table(
                    d.errors
                        .into_iter()
                        .map(move |e| UntaggedValue::Error(e).into_untagged_value())
                        .collect::<Vec<Value>>(),
                ),
                tag: d.tag.clone(),
            };
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
            UntaggedValue::path(f.path).retag(f.tag.clone()),
        );
        r.insert(
            "apparent".to_string(),
            UntaggedValue::bytes(f.size).retag(f.tag.clone()),
        );
        let b = match f.blocks {
            Some(k) => UntaggedValue::bytes(k).retag(f.tag.clone()),
            None => UntaggedValue::nothing().retag(f.tag.clone()),
        };
        r.insert("physical".to_string(), b);
        Value {
            value: UntaggedValue::row(r),
            tag: f.tag,
        }
    }
}
