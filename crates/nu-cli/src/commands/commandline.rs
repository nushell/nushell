use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::IntoPipelineData;
use nu_protocol::{PipelineData, ShellError, Signature, SyntaxShape, Type, Value};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct Commandline;

impl Command for Commandline {
    fn name(&self) -> &str {
        "commandline"
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch(
                "cursor",
                "Set or get the current cursor position",
                Some('c'),
            )
            .switch(
                "append",
                "appends the string to the end of the buffer",
                Some('a'),
            )
            .switch(
                "insert",
                "inserts the string into the buffer at the cursor position",
                Some('i'),
            )
            .switch(
                "replace",
                "replaces the current contents of the buffer (default)",
                Some('r'),
            )
            .optional(
                "cmd",
                SyntaxShape::String,
                "the string to perform the operation with",
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "View or modify the current command line input buffer."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        if let Some(cmd) = call.opt::<Value>(engine_state, stack, 0)? {
            let mut buffer = engine_state
                .repl_buffer_state
                .lock()
                .expect("repl buffer state mutex");
            let mut cursor_pos = engine_state
                .repl_cursor_pos
                .lock()
                .expect("repl cursor pos mutex");

            if call.has_flag("cursor") {
                let cmd_str = cmd.as_string()?;
                match cmd_str.parse::<i64>() {
                    Ok(n) => {
                        *cursor_pos = if n <= 0 {
                            0usize
                        } else {
                            buffer
                                .grapheme_indices(true)
                                .map(|(i, _c)| i)
                                .nth(n as usize)
                                .unwrap_or(buffer.len())
                        }
                    }
                    Err(_) => {
                        return Err(ShellError::CantConvert {
                            to_type: "int".to_string(),
                            from_type: "string".to_string(),
                            span: cmd.span()?,
                            help: Some(format!(
                                r#"string "{cmd_str}" does not represent a valid integer"#
                            )),
                        })
                    }
                }
            } else if call.has_flag("append") {
                buffer.push_str(&cmd.as_string()?);
            } else if call.has_flag("insert") {
                let cmd_str = cmd.as_string()?;
                buffer.insert_str(*cursor_pos, &cmd_str);
                *cursor_pos += cmd_str.len();
            } else {
                *buffer = cmd.as_string()?;
                *cursor_pos = buffer.len();
            }
            Ok(Value::Nothing { span: call.head }.into_pipeline_data())
        } else {
            let buffer = engine_state
                .repl_buffer_state
                .lock()
                .expect("repl buffer state mutex");
            if call.has_flag("cursor") {
                let cursor_pos = engine_state
                    .repl_cursor_pos
                    .lock()
                    .expect("repl cursor pos mutex");
                let char_pos = buffer
                    .grapheme_indices(true)
                    .chain(std::iter::once((buffer.len(), "")))
                    .position(|(i, _c)| i == *cursor_pos)
                    .expect("Cursor position isn't on a grapheme boundary");
                Ok(Value::String {
                    val: char_pos.to_string(),
                    span: call.head,
                }
                .into_pipeline_data())
            } else {
                Ok(Value::String {
                    val: buffer.to_string(),
                    span: call.head,
                }
                .into_pipeline_data())
            }
        }
    }
}
