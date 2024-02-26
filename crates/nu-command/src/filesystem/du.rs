use super::util::opt_for_glob_pattern;
use crate::{DirBuilder, DirInfo, FileInfo};
use nu_engine::{current_dir, CallExt};
use nu_glob::Pattern;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, NuGlob, PipelineData, ShellError, Signature,
    Span, Spanned, SyntaxShape, Type, Value,
};
use serde::Deserialize;

#[derive(Clone)]
pub struct Du;

#[derive(Deserialize, Clone, Debug)]
pub struct DuArgs {
    path: Option<Spanned<NuGlob>>,
    all: bool,
    deref: bool,
    exclude: Option<Spanned<String>>,
    #[serde(rename = "max-depth")]
    max_depth: Option<Spanned<i64>>,
    #[serde(rename = "min-size")]
    min_size: Option<Spanned<i64>>,
}

impl Command for Du {
    fn name(&self) -> &str {
        "du"
    }

    fn usage(&self) -> &str {
        "Find disk usage sizes of specified items."
    }

    fn signature(&self) -> Signature {
        Signature::build("du")
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .allow_variants_without_examples(true)
            .optional(
                "path",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
                "Starting directory.",
            )
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
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let tag = call.head;
        let min_size: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "min-size")?;
        let max_depth: Option<Spanned<i64>> = call.get_flag(engine_state, stack, "max-depth")?;
        if let Some(ref max_depth) = max_depth {
            if max_depth.item < 0 {
                return Err(ShellError::NeedsPositiveValue {
                    span: max_depth.span,
                });
            }
        }
        if let Some(ref min_size) = min_size {
            if min_size.item < 0 {
                return Err(ShellError::NeedsPositiveValue {
                    span: min_size.span,
                });
            }
        }
        let current_dir = current_dir(engine_state, stack)?;

        let args = DuArgs {
            path: opt_for_glob_pattern(engine_state, stack, call, 0)?,
            all: call.has_flag(engine_state, stack, "all")?,
            deref: call.has_flag(engine_state, stack, "deref")?,
            exclude: call.get_flag(engine_state, stack, "exclude")?,
            max_depth,
            min_size,
        };

        let exclude = args.exclude.map_or(Ok(None), move |x| {
            Pattern::new(&x.item)
                .map(Some)
                .map_err(|e| ShellError::InvalidGlobPattern {
                    msg: e.msg.into(),
                    span: x.span,
                })
        })?;

        let include_files = args.all;
        let mut paths = match args.path {
            Some(p) => nu_engine::glob_from(&p, &current_dir, call.head, None),
            // The * pattern should never fail.
            None => nu_engine::glob_from(
                &Spanned {
                    item: NuGlob::Expand("*".into()),
                    span: Span::unknown(),
                },
                &current_dir,
                call.head,
                None,
            ),
        }
        .map(|f| f.1)?
        .filter(move |p| {
            if include_files {
                true
            } else {
                matches!(p, Ok(f) if f.is_dir())
            }
        });

        let all = args.all;
        let deref = args.deref;
        let max_depth = args.max_depth.map(|f| f.item as u64);
        let min_size = args.min_size.map(|f| f.item as u64);

        let params = DirBuilder {
            tag,
            min: min_size,
            deref,
            exclude,
            all,
        };

        let mut output: Vec<Value> = vec![];
        for p in paths.by_ref() {
            match p {
                Ok(a) => {
                    if a.is_dir() {
                        output.push(
                            DirInfo::new(a, &params, max_depth, engine_state.ctrlc.clone()).into(),
                        );
                    } else if let Ok(v) = FileInfo::new(a, deref, tag) {
                        output.push(v.into());
                    }
                }
                Err(e) => {
                    output.push(Value::error(e, tag));
                }
            }
        }

        Ok(output.into_pipeline_data(engine_state.ctrlc.clone()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Disk usage of the current directory",
            example: "du",
            result: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::Du;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Du {})
    }
}
