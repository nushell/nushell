use super::state::stream_find;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxFind;

impl Command for IdxFind {
    fn name(&self) -> &str {
        "idx find"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("query", SyntaxShape::String, "Freeform fuzzy query.")
            .switch("verbose", "Include verbose scoring details.", Some('v'))
            .switch("dirs", "Search directories only.", Some('d'))
            .switch("files", "Search files only.", Some('f'))
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of rows to return.",
                Some('l'),
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Search idx with fuzzy matching across files and directories by default."
    }

    fn extra_description(&self) -> &str {
        "`idx find` searches both files and directories unless you narrow it with `--files` or `--dirs`."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Fuzzy search for files and directories matching 'main'",
                example: "idx find main",
                result: None,
            },
            Example {
                description: "Search only files with verbose scoring output",
                example: "idx find config --files --verbose",
                result: None,
            },
            Example {
                description: "Search only directories, limited to top 10 results",
                example: "idx find src --dirs --limit 10",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query: String = call.req(engine_state, stack, 0)?;
        let verbose = call.has_flag(engine_state, stack, "verbose")?;
        let dirs = call.has_flag(engine_state, stack, "dirs")?;
        let files = call.has_flag(engine_state, stack, "files")?;

        if files && dirs {
            return Err(ShellError::IncompatibleParameters {
                left_message: "--files cannot be used with --dirs".to_string(),
                left_span: call.get_flag_span(stack, "files").unwrap_or(call.head),
                right_message: "--dirs cannot be used with --files".to_string(),
                right_span: call.get_flag_span(stack, "dirs").unwrap_or(call.head),
            });
        }

        let limit = call
            .get_flag::<i64>(engine_state, stack, "limit")?
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(100);

        let signals = engine_state.signals();
        stream_find(&query, files, dirs, verbose, limit, call.head, signals)
    }
}
