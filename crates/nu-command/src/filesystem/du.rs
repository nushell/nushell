use crate::{DirBuilder, DirInfo, ExcludeGlob, FileId, FileInfo};
use nu_engine::command_prelude::*;
use nu_glob::MatchOptions;
use nu_protocol::{NuGlob, PipelineMetadata, Signals};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Du;

#[derive(Deserialize, Clone, Debug)]
pub struct DuArgs {
    path: Option<Spanned<NuGlob>>,
    deref: bool,
    long: bool,
    all: bool,
    exclude: Option<Spanned<NuGlob>>,
    #[serde(rename = "max-depth")]
    max_depth: Option<Spanned<i64>>,
    #[serde(rename = "min-size")]
    min_size: Option<Spanned<i64>>,
    #[serde(rename = "count-links")]
    count_links: bool,
}

impl Command for Du {
    fn name(&self) -> &str {
        "du"
    }

    fn description(&self) -> &str {
        "Find disk usage sizes of specified items."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["disk", "usage", "size", "space"]
    }

    fn signature(&self) -> Signature {
        Signature::build("du")
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
            .rest(
                "path",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]),
                "Starting directory.",
            )
            .switch(
                "deref",
                "Dereference symlinks to their targets for size.",
                Some('r'),
            )
            .switch(
                "long",
                "Get underlying directories and files for each entry.",
                Some('l'),
            )
            .named(
                "exclude",
                SyntaxShape::GlobPattern,
                "Exclude these file names.",
                Some('x'),
            )
            .named(
                "max-depth",
                SyntaxShape::Int,
                "Directory recursion limit.",
                Some('d'),
            )
            .named(
                "min-size",
                SyntaxShape::Int,
                "Exclude files below this size.",
                Some('m'),
            )
            .switch("all", "Include hidden files if '*' is provided.", Some('a'))
            // GNU and BSD both use '-l' for the short-form, which is already taken
            // by '--long' in nushell across multiple commands, so long-form flag only.
            .switch(
                "count-links",
                "Count sizes many times if hard linked.",
                None,
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
        if let Some(ref max_depth) = max_depth
            && max_depth.item < 0
        {
            return Err(ShellError::NeedsPositiveValue {
                span: max_depth.span,
            });
        }
        if let Some(ref min_size) = min_size
            && min_size.item < 0
        {
            return Err(ShellError::NeedsPositiveValue {
                span: min_size.span,
            });
        }
        let deref = call.has_flag(engine_state, stack, "deref")?;
        let long = call.has_flag(engine_state, stack, "long")?;
        let exclude = call.get_flag(engine_state, stack, "exclude")?;
        let current_dir = engine_state.cwd(Some(stack))?.into_std_path_buf();
        let all = call.has_flag(engine_state, stack, "all")?;
        let count_links = call.has_flag(engine_state, stack, "count-links")?;

        // One set of device-inode pairs for the whole command, shared across
        // every path operand, holding the files whose sizes have already been
        // counted. POSIX requires that reach: "a file that occurs multiple
        // times shall be counted and written for only one entry, even if the
        // occurrences are under different file operands". `du_for_one_pattern`
        // returns a lazy iterator that outlives this function, so the set is
        // shared by ownership rather than borrowed, and the `Mutex` is what
        // keeps that iterator `Send`.
        let counted = Arc::new(Mutex::new(HashSet::new()));

        let paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;
        let paths = if !call.has_positional_args(stack, 0) {
            None
        } else {
            Some(paths)
        };

        match paths {
            None => {
                let args = DuArgs {
                    path: None,
                    deref,
                    long,
                    all,
                    exclude,
                    max_depth,
                    min_size,
                    count_links,
                };
                Ok(du_for_one_pattern(
                    args,
                    &current_dir,
                    tag,
                    engine_state.signals().clone(),
                    counted,
                )?
                .into_pipeline_data_with_metadata(
                    tag,
                    engine_state.signals().clone(),
                    PipelineMetadata {
                        path_columns: vec![String::from("path")],
                        ..Default::default()
                    },
                ))
            }
            Some(paths) => {
                let mut result_iters = vec![];
                for p in paths {
                    let args = DuArgs {
                        path: Some(p),
                        deref,
                        long,
                        all,
                        exclude: exclude.clone(),
                        max_depth,
                        min_size,
                        count_links,
                    };
                    result_iters.push(du_for_one_pattern(
                        args,
                        &current_dir,
                        tag,
                        engine_state.signals().clone(),
                        Arc::clone(&counted),
                    )?)
                }

                // chain all iterators on result.
                Ok(result_iters
                    .into_iter()
                    .flatten()
                    .into_pipeline_data_with_metadata(
                        tag,
                        engine_state.signals().clone(),
                        PipelineMetadata {
                            path_columns: vec![String::from("path")],
                            ..Default::default()
                        },
                    ))
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Disk usage of the current directory.",
            example: "du",
            result: None,
        }]
    }
}

fn du_for_one_pattern(
    args: DuArgs,
    current_dir: &Path,
    span: Span,
    signals: Signals,
    counted: Arc<Mutex<HashSet<FileId>>>,
) -> Result<impl Iterator<Item = Value> + Send + use<>, ShellError> {
    let exclude = args
        .exclude
        .map(|x| build_exclude_glob(x.item.as_ref(), x.span))
        .transpose()?;
    let glob_options = if args.all {
        None
    } else {
        let glob_options = MatchOptions {
            require_literal_leading_dot: true,
            ..Default::default()
        };
        Some(glob_options)
    };
    let paths = match args.path {
        Some(p) => nu_engine::glob_from(&p, current_dir, span, glob_options, signals.clone()),

        // The * pattern should never fail.
        None => nu_engine::glob_from(
            &Spanned {
                item: NuGlob::Expand("*".into()),
                span,
            },
            current_dir,
            span,
            None,
            signals.clone(),
        ),
    }
    .map(|f| f.1)?;

    let deref = args.deref;
    let long = args.long;
    let max_depth = args.max_depth.map(|f| f.item as u64);
    let min_size = args.min_size.map(|f| f.item as u64);

    let params = DirBuilder {
        tag: span,
        min: min_size,
        deref,
        exclude,
        long,
        count_links: args.count_links,
    };

    Ok(paths.filter_map(move |p| match p {
        Ok(a) => {
            // Taken once per path operand, not once per file. The walk below
            // finishes before this closure returns its row, so the lock is
            // released before the iterator pauses and can never be held while
            // something else runs. A poisoned lock means only that an earlier
            // walk panicked; the set behind it is still an accurate record of
            // what has been counted, so carry on with it rather than failing
            // the whole command.
            let mut counted = counted.lock().unwrap_or_else(|e| e.into_inner());

            if a.is_dir() {
                match DirInfo::new(a, &params, max_depth, span, &signals, &mut counted) {
                    Ok(v) => Some(Value::from(v)),
                    Err(_) => None,
                }
            } else {
                match FileInfo::new(a, deref, span, params.long, params.count_links) {
                    // A file already counted under another name, or under
                    // another operand, is written only once.
                    Ok(v) if v.insert_into(&mut counted) => Some(Value::from(v)),
                    Ok(_) => None,
                    Err(_) => None,
                }
            }
        }
        Err(e) => Some(Value::error(e, span)),
    }))
}

fn build_exclude_glob(pattern: &str, span: Span) -> Result<ExcludeGlob, ShellError> {
    match nu_experimental::DC_GLOB.get() {
        true => nu_glob::dc_glob::DcPattern::new(pattern)
            .map(ExcludeGlob::DcGlob)
            .map_err(|e| ShellError::InvalidGlobPattern {
                msg: e.to_string(),
                span,
            }),
        false => nu_glob::Pattern::new(pattern)
            .map(ExcludeGlob::Legacy)
            .map_err(|e| ShellError::InvalidGlobPattern {
                msg: e.msg.into(),
                span,
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::Du;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(Du)
    }
}
