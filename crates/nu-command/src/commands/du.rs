use crate::prelude::*;
use glob::*;
use nu_engine::WholeStreamCommand;
use nu_engine::{DirBuilder, DirInfo, FileInfo};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;
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
            .optional("path", SyntaxShape::GlobPattern, "starting directory")
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
                SyntaxShape::GlobPattern,
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        du(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Disk usage of the current directory",
            example: "du",
            result: None,
        }]
    }
}

async fn du(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctrl_c = args.ctrl_c.clone();
    let ctrl_c_copy = ctrl_c.clone();

    let (args, _): (DuArgs, _) = args.process().await?;
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

fn glob_err_into(e: GlobError) -> ShellError {
    let e = e.into_error();
    ShellError::from(e)
}

#[cfg(test)]
mod tests {
    use super::Du;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Du {})
    }
}
