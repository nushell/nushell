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
                (Type::String, Type::Int),
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
            let span = cmd.span();
            let cmd = cmd.coerce_into_string()?;
            let mut repl = engine_state.repl_state.lock().expect("repl state mutex");

            if call.has_flag(engine_state, stack, "cursor")? {
                match cmd.parse::<i64>() {
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
                            span,
                            help: Some(format!(r#"string "{cmd}" does not represent a valid int"#)),
                        })
                    }
                }
            } else if call.has_flag(engine_state, stack, "append")? {
                repl.buffer.push_str(&cmd);
            } else if call.has_flag(engine_state, stack, "insert")? {
                let cursor_pos = repl.cursor_pos;
                repl.buffer.insert_str(cursor_pos, &cmd);
                repl.cursor_pos += cmd.len();
            } else {
                repl.buffer = cmd;
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
                match i64::try_from(char_pos) {
                    Ok(pos) => Ok(Value::int(pos, call.head).into_pipeline_data()),
                    Err(e) => Err(ShellError::GenericError {
                        error: "Failed to convert cursor position to int".to_string(),
                        msg: e.to_string(),
                        span: None,
                        help: None,
                        inner: vec![],
                    }),
                }
            } else {
                Ok(Value::string(repl.buffer.to_string(), call.head).into_pipeline_data())
            }
        }
    }
}
