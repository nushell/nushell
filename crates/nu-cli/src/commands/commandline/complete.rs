use std::borrow::Cow;

use nu_engine::command_prelude::*;
use nu_protocol::FromValue;

use crate::completions::{
    Buffer, Completer, CompletionEngine, DirectoryCompletion, FileCompletion, SemanticSuggestion,
};

#[derive(Debug, Clone, FromValue)]
#[nu_value(rename_all = "kebab-case", type_name = "directory | path | glob")]
enum CompletionType {
    Directory,
    Path,
    Glob,
}

#[derive(Clone)]
pub struct CommandlineComplete;

impl Command for CommandlineComplete {
    fn name(&self) -> &str {
        "commandline complete"
    }

    fn description(&self) -> &str {
        "Complete a string using the default completions."
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline complete")
            .input_output_type(
                Type::Nothing,
                Type::one_of([
                    Type::list(Type::String),
                    Type::list(Type::record()),
                    Type::record(),
                ]),
            )
            .input_output_type(
                Type::String,
                Type::one_of([
                    Type::list(Type::String),
                    Type::list(Type::record()),
                    Type::record(),
                ]),
            )
            .switch(
                "detailed",
                "Output completions as records, in the format expected from custom completers.",
                Some('d'),
            )
            .switch(
                "input",
                "Output the record a completer would receive here, instead of completions.",
                Some('i'),
            )
            .param(
                Flag::new("type")
                    .arg(SyntaxShape::String)
                    .desc("The type of values to allow as completions.")
                    .completion(Completion::List(nu_utils::NuCow::Borrowed(&[
                        "directory",
                        "glob",
                        "path",
                    ]))),
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        "This command can be used to obtain the completions that Nushell would normally provide for the given commandline contents.
Completions will be provided as if the cursor is placed at the end of the given string.

If no input is provided, the current commandline contents will be used instead.

With --input, the record a completer would receive at that position is returned instead of
completions, which is the supported way to develop and test a completer from inside Nushell."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive", "completion"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call_span = call.span();
        let head_span = call.head;
        let source_span = input.span().unwrap_or(head_span);

        let (buffer, cursor_position) =
            extract_input_buffer(&input, engine_state, head_span, source_span)?;

        let is_detailed = call.has_flag(engine_state, stack, "detailed")?;

        // `--input` returns the site, not completions; reject flags that shape output.
        if call.has_flag(engine_state, stack, "input")? {
            for conflicting in ["detailed", "type"] {
                if let Some(span) = call.get_flag_span(stack, conflicting) {
                    return Err(ShellError::IncompatibleParameters {
                        left_message: "cannot be used with --input".into(),
                        left_span: span,
                        right_message: "--input returns the completer's input record".into(),
                        right_span: call.get_flag_span(stack, "input").unwrap_or(head_span),
                    });
                }
            }

            return Ok(CompletionEngine::new(engine_state, stack)
                .completer_input_at(&buffer, cursor_position)
                .into_pipeline_data());
        }

        let completion_type = match call.get_flag::<Value>(engine_state, stack, "type")? {
            Some(v) => {
                let type_str = v
                    .as_str()
                    .map(nu_utils::escape_quote_string)
                    .unwrap_or_default();

                Some(CompletionType::from_value(v.clone()).map_err(|_| {
                    ShellError::InvalidValue {
                        valid: r#"type "directory", "path", or "glob""#.into(),
                        actual: type_str,
                        span: v.span(),
                    }
                })?)
            }
            None => None,
        };

        let completions = fetch_completions(
            engine_state,
            stack,
            completion_type,
            &buffer,
            cursor_position,
        );

        let result_values: Vec<Value> = completions
            .into_iter()
            .map(|suggestion| match is_detailed {
                true => suggestion.into_value(call_span),
                false => Value::string(suggestion.suggestion.value, call_span),
            })
            .collect();

        Ok(Value::list(result_values, call_span).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "List completions for command names.",
                example: "def my-bar [] {}; def my-baz [] {}; 'my-' | commandline complete",
                result: Some(Value::list(
                    vec![
                        Value::string("my-bar", Span::test_data()),
                        Value::string("my-baz", Span::test_data()),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "List completions for flags for a command.",
                example: "def cmd [--flag(-f): string] {}; 'cmd -' | commandline complete",
                result: Some(Value::list(
                    vec![
                        Value::string("--flag", Span::test_data()),
                        Value::string("-f", Span::test_data()),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Complete filepath or glob arguments.",
                example: "'./' | commandline complete --type path",
                result: None,
            },
            Example {
                description: "Extend builtin completions for the current commandline.",
                example: "commandline complete | append 'foo'",
                result: None,
            },
            Example {
                description: "Inspect what a completer would be handed at the cursor.",
                example: "'git checkout ma' | commandline complete --input | get site",
                result: None,
            },
        ]
    }
}

/// Read the buffer and cursor position from a string input or, if none, the repl state.
fn extract_input_buffer<'a>(
    input: &'a PipelineData,
    engine_state: &EngineState,
    head_span: Span,
    source_span: Span,
) -> Result<(Cow<'a, str>, usize), ShellError> {
    match input {
        PipelineData::Value(
            Value::String {
                val: string_value, ..
            },
            _,
        ) => Ok((Cow::Borrowed(string_value.as_str()), string_value.len())),
        PipelineData::Empty => {
            // Clone to avoid holding the lock while fetching completions, which may execute arbitrary code.
            let repl = engine_state.repl_state.lock().expect("repl state mutex");
            Ok((Cow::Owned(repl.buffer.clone()), repl.cursor_pos))
        }
        _ => Err(ShellError::PipelineMismatch {
            exp_input_type: "string or nothing".into(),
            dst_span: head_span,
            src_span: source_span,
        }),
    }
}

/// Fetch completions for `buffer` at `cursor_position`, type-restricted (`--type`) or
/// the default set for the whole line.
fn fetch_completions(
    engine_state: &EngineState,
    stack: &mut Stack,
    completion_type: Option<CompletionType>,
    buffer: &str,
    cursor_position: usize,
) -> Vec<SemanticSuggestion> {
    let completer = CompletionEngine::new(engine_state, stack);

    completion_type
        .map(|parsed_type| {
            generate_typed_suggestions(
                engine_state,
                &completer,
                parsed_type,
                buffer,
                cursor_position,
            )
        })
        .unwrap_or_else(|| completer.fetch_completions_at(buffer, cursor_position))
}

/// Completions restricted to a single [`CompletionType`] (directory/path/glob).
fn generate_typed_suggestions(
    engine_state: &EngineState,
    completer: &CompletionEngine,
    completion_type: CompletionType,
    buffer: &str,
    cursor_position: usize,
) -> Vec<SemanticSuggestion> {
    let mut working_set = StateWorkingSet::new(engine_state);
    let buffer_bytes = buffer.as_bytes();

    // Clamp `cursor_position` to a valid char boundary so the slice below is safe.
    let cursor_position = buffer.floor_char_boundary(cursor_position.min(buffer.len()));

    let file_id = working_set.add_file("completer", buffer_bytes);
    let file_span = working_set.get_span_for_file(file_id);

    let context = completer.context(
        &working_set,
        Buffer {
            text: &buffer[..cursor_position],
            offset: file_span.start,
        },
        file_span,
        &buffer_bytes[..cursor_position],
    );

    // Explicit matching avoids boxing the source into a `dyn` trait object.
    match completion_type {
        CompletionType::Directory => DirectoryCompletion.fetch(&context).into_suggestions(),
        CompletionType::Path | CompletionType::Glob => {
            FileCompletion.fetch(&context).into_suggestions()
        }
    }
}
