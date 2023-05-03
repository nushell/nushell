use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Spanned,
    SyntaxShape, Type, Value,
};
use wax::{Glob as WaxGlob, WalkBehavior};

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
        ]
    }

    fn extra_usage(&self) -> &str {
        r#"For more glob pattern help, please refer to https://github.com/olson-sean-k/wax"#
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
        let path = current_dir(engine_state, stack)?;
        let glob_pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
        let depth = call.get_flag(engine_state, stack, "depth")?;

        let no_dirs = call.has_flag("no-dir");
        let no_files = call.has_flag("no-file");
        let no_symlinks = call.has_flag("no-symlink");

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

        let glob = match WaxGlob::new(&glob_pattern.item) {
            Ok(p) => p,
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

        #[allow(clippy::needless_collect)]
        let glob_results = glob
            .walk_with_behavior(
                path,
                WalkBehavior {
                    depth: folder_depth,
                    ..Default::default()
                },
            )
            .flatten();
        let mut result: Vec<Value> = Vec::new();
        for entry in glob_results {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                result.clear();
                break;
            }
            let file_type = entry.file_type();

            if !(no_dirs && file_type.is_dir()
                || no_files && file_type.is_file()
                || no_symlinks && file_type.is_symlink())
            {
                result.push(Value::String {
                    val: entry.into_path().to_string_lossy().to_string(),
                    span,
                });
            }
        }

        Ok(result
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}
