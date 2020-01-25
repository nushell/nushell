use crate::commands::command::RunnablePerItemContext;
use crate::prelude::*;
use glob::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use std::fs;
use std::path::PathBuf;

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
    max_depth: Option<u64>,
    min_size: Option<u64>,
}

impl PerItemCommand for Du {
    fn name(&self) -> &str {
        NAME
    }

    fn signature(&self) -> Signature {
        Signature::build(NAME)
            .optional("path", SyntaxShape::Pattern, "starting directory")
            .switch("all", "Output File sizes as well as directory sizes")
            .switch("deref", "Dereference symlinks to their targets for size")
            .named("exclude", SyntaxShape::Pattern, "Exclude these file names")
            .named("max-depth", SyntaxShape::Int, "Directory recursion limit")
            .named(
                "min-size",
                SyntaxShape::Int,
                "Exclude files below this size",
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
    let all = args.all;
    let deref = args.deref;

    let exclude = args
        .exclude
        .as_ref()
        .map_or(Ok(None), |x| match Pattern::new(&x.item) {
            Ok(p) => Ok(Some(p)),
            Err(e) => Err(ShellError::labeled_error(
                e.msg,
                "Glob error",
                x.tag.clone(),
            )),
        })?;
    let max_depth = args.max_depth;
    let min_size = args.min_size;

    let paths = match args.path.as_ref() {
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
            Err(e) => Err(ShellError::labeled_error(
                e.msg,
                "Glob error",
                ctx.name.clone(),
            )),
        },
    }?
    .filter(|p| {
        if args.path.is_none() {
            match p {
                Ok(f) if f.is_dir() => true,
                Err(e) if e.path().is_dir() => true,
                _ => false,
            }
        } else {
            true
        }
    })
    .map(|p| match p {
        Err(e) => Err(glob_err_into(e)),
        Ok(s) => Ok(s),
    });

    let paths = paths.filter_map(|p| match p {
        Ok(p) => match get_info(p, deref, &min_size, &exclude, max_depth, all) {
            Ok(Some(mut d)) => {
                d.set_tag(ctx.name.clone());
                Some(Ok(ReturnSuccess::Value(d.into())))
            }
            Err(e) => Some(Err(e)),
            _ => None,
        },
        Err(e) => Some(Err(e)),
    });

    Ok(paths.collect::<Vec<ReturnValue>>().into())
}

fn get_info(
    path: PathBuf,
    deref: bool,
    min_size: &Option<u64>,
    exclude: &Option<Pattern>,
    depth: Option<u64>,
    all: bool,
) -> Result<Option<Info>, ShellError> {
    if depth.as_ref().map_or(false, |d| *d == 0) {
        Ok(None)
    } else if path.is_dir() {
        match fs::read_dir(&path) {
            Ok(d) => {
                let mut info = Info::new(path.to_str().expect("This should be encoded properly"));
                for file in d {
                    match file {
                        Ok(e) => {
                            match get_info(
                                e.path(),
                                deref,
                                min_size,
                                exclude,
                                depth.map(|d| d - 1),
                                all,
                            ) {
                                Ok(Some(i)) => {
                                    info.size += &i.size;
                                    info.add_sub(e.file_name().to_string_lossy().into(), i);
                                }
                                Ok(None) => continue,
                                Err(e) => info.add_err(e),
                            }
                        }
                        Err(e) => info.add_err(ShellError::from(e)),
                    }
                }
                Ok(Some(info))
            }
            Err(e) => Err(ShellError::from(e)),
        }
    } else {
        if exclude.as_ref().map_or(false, |x| {
            x.matches(
                path.file_name()
                    .expect("How would this even happen?")
                    .to_str()
                    .expect("Very invalid filename apparently?"),
            )
        }) {
            Ok(None)
        } else {
            match fs::metadata(&path) {
                Ok(m) => {
                    let size = if !deref {
                        match fs::symlink_metadata(&path) {
                            Ok(s) => Ok(s.len()),
                            Err(e) => Err(ShellError::from(e)),
                        }
                    } else {
                        Ok(m.len())
                    }?;
                    Ok(Some(Info::new_file(path.to_string_lossy(), size)))
                }
                Err(e) => Err(ShellError::from(e)),
            }
        }
    }
}

struct Info {
    sub: IndexMap<String, Info>,
    errors: Vec<Value>,
    size: u64,
    name: String,
    tag: Tag,
}

impl Info {
    fn new(name: impl Into<String>) -> Self {
        Info {
            sub: IndexMap::new(),
            errors: Vec::new(),
            size: 0,
            name: name.into(),
            tag: Tag::unknown(),
        }
    }

    fn new_file(name: impl Into<String>, size: u64) -> Info {
        let mut new = Info::new(name);
        new.size = size;
        new
    }
    fn add_sub(&mut self, s: String, i: Info) {
        self.sub.insert(s, i);
    }

    fn add_err(&mut self, e: impl Into<UntaggedValue>) {
        let v = e.into().into_untagged_value();
        self.errors.push(v);
    }

    fn set_tag(&mut self, t: Tag) {
        self.tag = t;
    }
}

fn glob_err_into(e: GlobError) -> ShellError {
    let e = e.into_error();
    ShellError::from(e)
}

impl From<Info> for Value {
    fn from(i: Info) -> Self {
        let n = i.name;
        let s = i.size;
        let mut subs: Vec<Value> = Vec::new();
        let mut row: IndexMap<String, Value> = IndexMap::new();
        row.insert(
            "name".to_string(),
            UntaggedValue::string(n).into_untagged_value(),
        );
        row.insert(
            "size".to_string(),
            UntaggedValue::bytes(s).into_untagged_value(),
        );
        for (_k, v) in i.sub {
            subs.push(v.into());
        }
        row.insert(
            "contents".to_string(),
            UntaggedValue::Table(subs).into_untagged_value(),
        );
        if !i.errors.is_empty() {
            row.insert(
                "errors".to_string(),
                UntaggedValue::Table(i.errors.into()).into_untagged_value(),
            );
        }

        UntaggedValue::row(row).into_untagged_value()
    }
}
