use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
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
            .required("glob", SyntaxShape::String, "the glob expression")
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
                "not",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "DEPRECATED OPTION: Patterns to exclude from the results",
                Some('n'),
            )
            .named(
                "prune",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Patterns to prune from the results: `glob` will not walk the inside of directories matching the prune patterns.",
                Some('p'),
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
                    "Search for files and folders that begin with uppercase C and lowercase c",
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
                example: r#"glob **/tsconfig.json --not [**/node_modules/**]"#,
                result: None,
            },
            Example {
                description: "Search for all files that are not in the target nor .git directories",
                example: r#"glob **/* --not [**/target/** **/.git/** */]"#,
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
        let no_dirs = call.has_flag("no-dir");
        let no_files = call.has_flag("no-file");
        let no_symlinks = call.has_flag("no-symlink");

        if call.has_flag("not") {
            nu_protocol::report_error_new(
                engine_state,
                &ShellError::GenericError(
                    "Deprecated option".into(),
                    "`glob --prune {list<string>}` is deprecated and will be removed in 0.88."
                        .into(),
                    Some(call.head),
                    Some("Please use `glob --prune {list<string>}` instead.".into()),
                    vec![],
                ),
            );
        }

        let not_flag: Option<Value> = call.get_flag(engine_state, stack, "not")?;
        let prune_flag: Option<Value> = call.get_flag(engine_state, stack, "prune")?;

        let paths_to_prune = match (not_flag, prune_flag) {
            (Some(not_flag), Some(prune_flag)) => {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "Cannot pass --not".into(),
                    left_span: not_flag.span(),
                    right_message: "and --prune".into(),
                    right_span: prune_flag.span(),
                })
            }
            (Some(not_flag), None) => Some(not_flag),
            (None, Some(prune_flag)) => Some(prune_flag),
            (None, None) => None,
        };

        let (not_patterns, not_pattern_span): (Vec<String>, Span) = match paths_to_prune {
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
            return Err(ShellError::GenericError(
                "glob pattern must not be empty".to_string(),
                "glob pattern is empty".to_string(),
                Some(glob_pattern.span),
                Some("add characters to the glob pattern".to_string()),
                Vec::new(),
            ));
        }

        let folder_depth = if let Some(depth) = depth {
            depth
        } else {
            usize::MAX
        };

        let (prefix, glob) = match WaxGlob::new(&glob_pattern.item) {
            Ok(p) => p.partition(),
            Err(e) => {
                return Err(ShellError::GenericError(
                    "error with glob pattern".to_string(),
                    format!("{e}"),
                    Some(glob_pattern.span),
                    None,
                    Vec::new(),
                ))
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
                return Err(ShellError::GenericError(
                    "error in canonicalize".to_string(),
                    format!("{e}"),
                    Some(glob_pattern.span),
                    None,
                    Vec::new(),
                ))
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
                .map_err(|err| {
                    ShellError::GenericError(
                        "error with glob's not pattern".to_string(),
                        format!("{err}"),
                        Some(not_pattern_span),
                        None,
                        Vec::new(),
                    )
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
