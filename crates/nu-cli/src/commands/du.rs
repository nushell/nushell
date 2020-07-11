use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use filesize::file_real_size_fast;
use glob::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
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

#[async_trait]
impl WholeStreamCommand for Du {
    fn name(&self) -> &str {
        NAME
    }

    fn signature(&self) -> Signature {
        Signature::build(NAME)
            .optional("path", SyntaxShape::Pattern, "starting directory")
            .switch(
                "all",
                "Output file sizes as well as directory sizes",
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        du(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Disk usage of the current directory",
            example: "du",
            result: None,
        }]
    }
}

async fn du(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let tag = args.call_info.name_tag.clone();
    let ctrl_c = args.ctrl_c.clone();
    let ctrl_c_copy = ctrl_c.clone();

    let (args, _): (DuArgs, _) = args.process(&registry).await?;
    let exclude = args.exclude.map_or(Ok(None), move |x| {
        Pattern::new(&x.item)
            .map(Option::Some)
            .map_err(|e| ShellError::labeled_error(e.msg, "glob error", x.tag.clone()))
    })?;

    let include_files = args.all;
    let paths = match args.path {
        Some(p) => {
            let p = p.item.to_str().expect("Why isn't this encoded properly?");
            glob::glob_with(p, GLOB_PARAMS)
        }
        None => glob::glob_with("*", GLOB_PARAMS),
    }
    .map_err(|e| ShellError::labeled_error(e.msg, "glob error", tag.clone()))?
    .filter(move |p| {
        if include_files {
            true
        } else {
            match p {
                Ok(f) if f.is_dir() => true,
                Err(e) if e.path().is_dir() => true,
                _ => false,
            }
        }
    })
    .map(|v| v.map_err(glob_err_into));

    let all = args.all;
    let deref = args.deref;
    let max_depth = args.max_depth.map(|f| f.item);
    let min_size = args.min_size.map(|f| f.item);

    let params = DirBuilder {
        tag: tag.clone(),
        min: min_size,
        deref,
        exclude,
        all,
    };

    let inp = futures::stream::iter(paths);

    Ok(inp
        .flat_map(move |path| match path {
            Ok(p) => {
                let mut output = vec![];
                if p.is_dir() {
                    output.push(Ok(ReturnSuccess::Value(
                        DirInfo::new(p, &params, max_depth, ctrl_c.clone()).into(),
                    )));
                } else {
                    for v in FileInfo::new(p, deref, tag.clone()).into_iter() {
                        output.push(Ok(ReturnSuccess::Value(v.into())));
                    }
                }
                futures::stream::iter(output)
            }
            Err(e) => futures::stream::iter(vec![Err(e)]),
        })
        .interruptible(ctrl_c_copy)
        .to_output_stream())
}

pub struct DirBuilder {
    tag: Tag,
    min: Option<u64>,
    deref: bool,
    exclude: Option<Pattern>,
    all: bool,
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
                                s = s.add_dir(i.path(), depth, &params, ctrl_c.clone())
                            }
                            Ok(_t) => s = s.add_file(i.path(), &params),
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

        let d = DirInfo::new(path, &params, depth, ctrl_c);
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

fn glob_err_into(e: GlobError) -> ShellError {
    let e = e.into_error();
    ShellError::from(e)
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

impl From<DirInfo> for Value {
    fn from(d: DirInfo) -> Self {
        let mut r: IndexMap<String, Value> = IndexMap::new();

        r.insert(
            "path".to_string(),
            UntaggedValue::path(d.path).into_value(&d.tag),
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
            UntaggedValue::path(f.path).into_value(&f.tag),
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

#[cfg(test)]
mod tests {
    use super::Du;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Du {})
    }
}
