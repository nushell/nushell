// use double_echo::Command as DoubleEcho;
// use double_ls::Command as DoubleLs;

use nu_command::commands::{
    Append, BuildString, Each, Echo, First, Get, Keep, Last, Let, Ls, Nth, RunExternalCommand,
    Select, StrCollect, Wrap,
};
use nu_engine::basic_evaluation_context;
use nu_engine::{whole_stream_command, EvaluationContext};

pub fn get_test_context() -> EvaluationContext {
    let base_context = basic_evaluation_context().expect("Could not create test context");

    base_context.add_commands(vec![
        // Minimal restricted commands to aid in testing
        whole_stream_command(Echo {}),
        whole_stream_command(RunExternalCommand { interactive: true }),
        whole_stream_command(Ls {}),
        whole_stream_command(Append {}),
        whole_stream_command(BuildString {}),
        whole_stream_command(First {}),
        whole_stream_command(Get {}),
        whole_stream_command(Keep {}),
        whole_stream_command(Each {}),
        whole_stream_command(Last {}),
        whole_stream_command(Nth {}),
        whole_stream_command(Let {}),
        whole_stream_command(Select),
        whole_stream_command(StrCollect),
        whole_stream_command(Wrap),
    ]);

    base_context
}
