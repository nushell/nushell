use nu_cli::eval_source2;
use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, ShellError,
};

pub fn run_nu_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    cmd: &str,
    current: PipelineData,
) -> std::result::Result<PipelineData, ShellError> {
    let mut engine_state = engine_state.clone();
    eval_source2(&mut engine_state, stack, cmd.as_bytes(), "", current)
}
