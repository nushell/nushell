use nu_engine::{command_prelude::*, env::current_dir};
use std::sync::{atomic::AtomicBool, Arc};
use wax::{Glob as WaxGlob, WalkBehavior, WalkEntry};

#[derive(Clone)]
pub struct Glob;

impl Command for Glob {
    fn name(&self) -> &str {
        "glob"
    }

    fn signature(&self) -> Signature {
        Signature::build("glob")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .required("glob", SyntaxShape::String, "The glob expression.")
            .named(
                "depth",
                SyntaxShape::Int,
                "directory depth to search",
                Some('d'),
            )
            .switch(
                "no-dir",
                "Whether to filter out directories from the returned paths",
                Some('D'),
            )
            .switch(
                "no-file",
                "Whether to filter out files from the returned paths",
                Some('F'),
            )
            .switch(
                "no-symlink",
                "Whether to filter out symlinks from the returned paths",
                Some('S'),
            )
            .named(
                "exclude",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Patterns to exclude from the search: `glob` will not walk the inside of directories matching the excluded patterns.",
                Some('e'),
            )
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
        "Creates a list of files and/or folders based on the glob pattern provided."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "files", "folders", "list", "ls"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for *.rs files",
                example: "glob *.rs",
                result: None,
            },
            Example {
                description: "Search for *.rs and *.toml files recursively up to 2 folders deep",
                example: "glob **/*.{rs,toml} --depth 2",
                result: None,
            },
            Example {
                description:
                    "Search for files and folders that begin with uppercase C or lowercase c",
                example: r#"glob "[Cc]*""#,
                result: None,
            },
            Example {
                description:
                    "Search for files and folders like abc or xyz substituting a character for ?",
                example: r#"glob "{a?c,x?z}""#,
                result: None,
            },
            Example {
                description: "A case-insensitive search for files and folders that begin with c",
                example: r#"glob "(?i)c*""#,
                result: None,
            },
            Example {
                description: "Search for files for folders that do not begin with c, C, b, M, or s",
                example: r#"glob "[!cCbMs]*""#,
                result: None,
            },
            Example {
                description: "Search for files or folders with 3 a's in a row in the name",
                example: "glob <a*:3>",
                result: None,
            },
            Example {
                description: "Search for files or folders with only a, b, c, or d in the file name between 1 and 10 times",
                example: "glob <[a-d]:1,10>",
                result: None,
            },
            Example {
                description: "Search for folders that begin with an uppercase ASCII letter, ignoring files and symlinks",
                example: r#"glob "[A-Z]*" --no-file --no-symlink"#,
                result: None,
            },
            Example {
                description: "Search for files named tsconfig.json that are not in node_modules directories",
                example: r#"glob **/tsconfig.json --exclude [**/node_modules/**]"#,
                result: None,
            },
            Example {
                description: "Search for all files that are not in the target nor .git directories",
                example: r#"glob **/* --exclude [**/target/** **/.git/** */]"#,
                result: None,
            },
        ]
    }

    fn extra_usage(&self) -> &str {
        r#"For more glob pattern help, please refer to https://docs.rs/crate/wax/latest"#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let ctrlc = engine_state.ctrlc.clone();
        let span = call.head;
        let glob_pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
        let depth = call.get_flag(engine_state, stack, "depth")?;
        let no_dirs = call.has_flag(engine_state, stack, "no-dir")?;
        let no_files = call.has_flag(engine_state, stack, "no-file")?;
        let no_symlinks = call.has_flag(engine_state, stack, "no-symlink")?;

        let paths_to_exclude: Option<Value> = call.get_flag(engine_state, stack, "exclude")?;

        let (not_patterns, not_pattern_span): (Vec<String>, Span) = match paths_to_exclude {
            None => (vec![], span),
            Some(f) => {
                let pat_span = f.span();
                match f {
                    Value::List { vals: pats, .. } => {
                        let p = convert_patterns(pats.as_slice())?;
                        (p, pat_span)
                    }
                    _ => (vec![], span),
                }
            }
        };

        if glob_pattern.item.is_empty() {
            return Err(ShellError::GenericError {
                error: "glob pattern must not be empty".into(),
                msg: "glob pattern is empty".into(),
                span: Some(glob_pattern.span),
                help: Some("add characters to the glob pattern".into()),
                inner: vec![],
            });
        }

        let folder_depth = if let Some(depth) = depth {
            depth
        } else {
            usize::MAX
        };

        let (prefix, glob) = match WaxGlob::new(&glob_pattern.item) {
            Ok(p) => p.partition(),
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "error with glob pattern".into(),
                    msg: format!("{e}"),
                    span: Some(glob_pattern.span),
                    help: None,
                    inner: vec![],
                })
            }
        };

        let path = current_dir(engine_state, stack)?;
        let path = match nu_path::canonicalize_with(prefix, path) {
            Ok(path) => path,
            Err(e) if e.to_string().contains("os error 2") =>
            // path we're trying to glob doesn't exist,
            {
                std::path::PathBuf::new() // user should get empty list not an error
            }
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "error in canonicalize".into(),
                    msg: format!("{e}"),
                    span: Some(glob_pattern.span),
                    help: None,
                    inner: vec![],
                })
            }
        };

        Ok(if !not_patterns.is_empty() {
            let np: Vec<&str> = not_patterns.iter().map(|s| s as &str).collect();
            let glob_results = glob
                .walk_with_behavior(
                    path,
                    WalkBehavior {
                        depth: folder_depth,
                        ..Default::default()
                    },
                )
                .not(np)
                .map_err(|err| ShellError::GenericError {
                    error: "error with glob's not pattern".into(),
                    msg: format!("{err}"),
                    span: Some(not_pattern_span),
                    help: None,
                    inner: vec![],
                })?
                .flatten();
            let result = glob_to_value(ctrlc, glob_results, no_dirs, no_files, no_symlinks, span)?;
            result
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone())
        } else {
            let glob_results = glob
                .walk_with_behavior(
                    path,
                    WalkBehavior {
                        depth: folder_depth,
                        ..Default::default()
                    },
                )
                .flatten();
            let result = glob_to_value(ctrlc, glob_results, no_dirs, no_files, no_symlinks, span)?;
            result
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone())
        })
    }
}

fn convert_patterns(columns: &[Value]) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value {
            Value::String { val: s, .. } => Ok(s.clone()),
            _ => Err(ShellError::IncompatibleParametersSingle {
                msg: "Incorrect column format, Only string as column name".to_string(),
                span: value.span(),
            }),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

fn glob_to_value<'a>(
    ctrlc: Option<Arc<AtomicBool>>,
    glob_results: impl Iterator<Item = WalkEntry<'a>>,
    no_dirs: bool,
    no_files: bool,
    no_symlinks: bool,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut result: Vec<Value> = Vec::new();
    for entry in glob_results {
        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
            result.clear();
            return Err(ShellError::InterruptedByUser { span: None });
        }
        let file_type = entry.file_type();

        if !(no_dirs && file_type.is_dir()
            || no_files && file_type.is_file()
            || no_symlinks && file_type.is_symlink())
        {
            result.push(Value::string(
                entry.into_path().to_string_lossy().to_string(),
                span,
            ));
        }
    }

    Ok(result)
}
