use nu_engine::command_prelude::*;
use reedline::ReedlineEvent;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "commandline edit"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
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
            .switch("accept", "instantly executes given command", Some('A'))
            .required(
                "str",
                SyntaxShape::String,
                "the string to perform the operation with",
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Modify the current command line input buffer."
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
        let str: String = call.req(engine_state, stack, 0)?;
        let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
        if call.has_flag(engine_state, stack, "append")? {
            repl.buffer.push_str(&str);
        } else if call.has_flag(engine_state, stack, "insert")? {
            let cursor_pos = repl.cursor_pos;
            repl.buffer.insert_str(cursor_pos, &str);
            repl.cursor_pos += str.len();
        } else {
            // default == "replace"
            repl.buffer = str;
            repl.cursor_pos = repl.buffer.len();
        }
        if call.has_flag(engine_state, stack, "accept")? {
            if let Some(sender) = &engine_state.reedline_event_sender {
                if let Err(e) = sender.send(ReedlineEvent::Submit) {
                    log::warn!("Could not send Submit to reedline: {e:?}");
                }
            }
        }
        Ok(Value::nothing(call.head).into_pipeline_data())
    }
}
