use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct Commandline;

impl Command for Commandline {
    fn name(&self) -> &str {
        "commandline"
    }

    fn signature(&self) -> Signature {
        Signature::build("commandline")
            .input_output_types(vec![
                (Type::Nothing, Type::Nothing),
                (Type::String, Type::String),
            ])
            .switch(
                "cursor",
                "Set or get the current cursor position",
                Some('c'),
            )
            .switch(
                "cursor-end",
                "Set the current cursor position to the end of the buffer",
                Some('e'),
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
            let mut repl = engine_state.repl_state.lock().expect("repl state mutex");

            if call.has_flag(engine_state, stack, "cursor")? {
                let cmd_str = cmd.as_string()?;
                match cmd_str.parse::<i64>() {
                    Ok(n) => {
                        repl.cursor_pos = if n <= 0 {
                            0usize
                        } else {
                            repl.buffer
                                .grapheme_indices(true)
                                .map(|(i, _c)| i)
                                .nth(n as usize)
                                .unwrap_or(repl.buffer.len())
                        }
                    }
                    Err(_) => {
                        return Err(ShellError::CantConvert {
                            to_type: "int".to_string(),
                            from_type: "string".to_string(),
                            span: cmd.span(),
                            help: Some(format!(
                                r#"string "{cmd_str}" does not represent a valid int"#
                            )),
                        })
                    }
                }
            } else if call.has_flag(engine_state, stack, "append")? {
                repl.buffer.push_str(&cmd.as_string()?);
            } else if call.has_flag(engine_state, stack, "insert")? {
                let cmd_str = cmd.as_string()?;
                let cursor_pos = repl.cursor_pos;
                repl.buffer.insert_str(cursor_pos, &cmd_str);
                repl.cursor_pos += cmd_str.len();
            } else {
                repl.buffer = cmd.as_string()?;
                repl.cursor_pos = repl.buffer.len();
            }
            Ok(Value::nothing(call.head).into_pipeline_data())
        } else {
            let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
            if call.has_flag(engine_state, stack, "cursor-end")? {
                repl.cursor_pos = repl.buffer.len();
                Ok(Value::nothing(call.head).into_pipeline_data())
            } else if call.has_flag(engine_state, stack, "cursor")? {
                let char_pos = repl
                    .buffer
                    .grapheme_indices(true)
                    .chain(std::iter::once((repl.buffer.len(), "")))
                    .position(|(i, _c)| i == repl.cursor_pos)
                    .expect("Cursor position isn't on a grapheme boundary");
                Ok(Value::string(char_pos.to_string(), call.head).into_pipeline_data())
            } else {
                Ok(Value::string(repl.buffer.to_string(), call.head).into_pipeline_data())
            }
        }
    }
}
