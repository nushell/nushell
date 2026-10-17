use nu_protocol::engine::EngineState;
use nu_protocol::{ShellWarning, report_shell_warning};
use std::sync::atomic::Ordering;

/// Exit the process or clean jobs if appropriate.
///
/// In an interactive session, warns if any jobs are not `auto_kill` and
/// `exit_warning_given` is false. Sets that flag and returns `tag` in that case.
/// Otherwise, kills all jobs, drops `tag`, and exits the current process.
///
// Currently, this `tag` argument exists mostly so that a LineEditor can be dropped before exiting the process.
pub fn cleanup_exit<T>(tag: T, engine_state: &EngineState, exit_code: i32) -> T {
    if let Some(tag) = cleanup(tag, engine_state) {
        return tag;
    }

    std::process::exit(exit_code);
}

/// Clean jobs if appropriate.
///
/// In an interactive session, warns if any jobs are not `auto_kill` and
/// `exit_warning_given` is false. Sets that flag and returns `Some(tag)` in that case.
/// Otherwise, kills all jobs, drops `tag`, and returns `None`.
pub fn cleanup<T>(tag: T, engine_state: &EngineState) -> Option<T> {
    let mut jobs = engine_state.jobs.lock().expect("failed to lock job table");

    let job_count = jobs.iter().filter(|(_, job)| !job.is_auto_kill()).count();

    if engine_state.is_interactive
        && job_count > 0
        && !engine_state.exit_warning_given.load(Ordering::SeqCst)
    {
        report_shell_warning(
            None,
            engine_state,
            &ShellWarning::BackgroundJobsRunning { job_count },
        );

        engine_state
            .exit_warning_given
            .store(true, Ordering::SeqCst);

        return Some(tag);
    }

    let _ = jobs.kill_all();

    drop(tag);
    None
}
