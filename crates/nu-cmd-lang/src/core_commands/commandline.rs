use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::ReplOperation;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::IntoPipelineData;
use nu_protocol::{PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

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
        "View or modify the current command line input buffer"
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
            let mut ops = engine_state
                .repl_operation_queue
                .lock()
                .expect("repl op queue mutex");
            ops.push_back(if call.has_flag("append") {
                ReplOperation::Append(cmd.as_string()?)
            } else if call.has_flag("insert") {
                ReplOperation::Insert(cmd.as_string()?)
            } else {
                ReplOperation::Replace(cmd.as_string()?)
            });
            Ok(Value::Nothing { span: call.head }.into_pipeline_data())
        } else if let Some(ref cmd) = *engine_state
            .repl_buffer_state
            .lock()
            .expect("repl buffer state mutex")
        {
            Ok(Value::String {
                val: cmd.clone(),
                span: call.head,
            }
            .into_pipeline_data())
        } else {
            Ok(Value::Nothing { span: call.head }.into_pipeline_data())
        }
    }
}
