use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};

// use crate::repl::{update_history_id_in_engine, HistorySessionIdPublic};

#[derive(Clone)]
pub struct HistorySession;

impl Command for HistorySession {
    fn name(&self) -> &str {
        "history session"
    }

    fn usage(&self) -> &str {
        "Get the command history session."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("history session")
            // .named("set", SyntaxShape::Int, "Set the session_id to", Some('s'))
            .category(Category::Misc)
            // .input_output_types(vec![(Type::Nothing, Type::Int), (Type::Int, Type::Nothing)])
            .input_output_types(vec![(Type::Nothing, Type::Int)])
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "history session",
                description: "Get current history session",
                result: None,
            },
            Example {
                example: "history -l | where session_id == (history session) | last 5",
                description: "Gets the last 5 history entries of the current session",
                result: None,
            },
            // Example {
            //     example: "history session --set (history session)",
            //     description: "Sets the history session to a different history session (example
            // sets it to the same history session)",     result: None,
            // },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // let set_session: Option<Value> = call.get_flag(engine_state, stack, "set")?;
        // if let Some(set_session) = set_session {
        //     let set_session_id = set_session.as_i64()?;
        //     #[allow(mutable_transmutes)]
        //     let engine_state =
        //         unsafe { std::mem::transmute::<&EngineState, &mut
        // EngineState>(engine_state) };

        //     update_history_id_in_engine(
        //         engine_state,
        //         todo!("Get line_editor here"),
        //         Some(HistorySessionIdPublic(set_session_id).into()),
        //     );
        //     engine_state.history_session_id = set_session_id;
        //     Ok(Value::nothing(call.head).into_pipeline_data())
        // } else {
        //     Ok(Value::int(engine_state.history_session_id,
        // call.head).into_pipeline_data()) }
        Ok(Value::int(engine_state.history_session_id, call.head).into_pipeline_data())
    }
}
