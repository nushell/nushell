use super::state::stream_grep;
use fff_search::GrepMode;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct IdxSearch;

impl Command for IdxSearch {
    fn name(&self) -> &str {
        "idx search"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "pattern",
                SyntaxShape::String,
                "One or more search patterns.",
            )
            .switch("regex", "Use regular-expression matching mode.", Some('r'))
            .switch("fuzzy", "Use fuzzy line-matching mode.", Some('f'))
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of matches to collect.",
                Some('l'),
            )
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::record())))])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Search indexed file contents."
    }

    fn extra_description(&self) -> &str {
        "Mode selection: plain text is the default and treats each pattern literally, `--regex` evaluates the patterns as regular expressions, and `--fuzzy` performs approximate line matching."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Search indexed file contents for a plain text pattern",
                example: "idx search hello",
                result: None,
            },
            Example {
                description: "Search using a regular expression",
                example: "idx search --regex 'fn \\w+'",
                result: None,
            },
            Example {
                description: "Search with multiple patterns simultaneously",
                example: "idx search TODO FIXME HACK",
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
        let patterns: Vec<String> = call.rest(engine_state, stack, 0)?;
        if patterns.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "pattern".to_string(),
                span: call.head,
            });
        }

        let regex = call.has_flag(engine_state, stack, "regex")?;
        let fuzzy = call.has_flag(engine_state, stack, "fuzzy")?;

        if regex && fuzzy {
            return Err(ShellError::IncompatibleParameters {
                left_message: "--regex cannot be used with --fuzzy".to_string(),
                left_span: call.get_flag_span(stack, "regex").unwrap_or(call.head),
                right_message: "--fuzzy cannot be used with --regex".to_string(),
                right_span: call.get_flag_span(stack, "fuzzy").unwrap_or(call.head),
            });
        }

        let limit = call
            .get_flag::<i64>(engine_state, stack, "limit")?
            .and_then(|v| usize::try_from(v).ok())
            .unwrap_or(50);

        let mode = if fuzzy {
            GrepMode::Fuzzy
        } else if regex {
            GrepMode::Regex
        } else {
            GrepMode::PlainText
        };

        let signals = engine_state.signals();
        stream_grep(&patterns, mode, limit, call.head, signals)
    }
}
