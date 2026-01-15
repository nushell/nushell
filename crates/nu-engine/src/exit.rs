use std::sync::atomic::Ordering;

use nu_protocol::engine::EngineState;

/// Request an exit from within a command pipeline.
///
/// This is called by the `exit` command when running in interactive mode.
/// Instead of calling `std::process::exit()` directly (which would skip history saving),
/// this function sets a flag that the REPL loop checks after command execution.
///
/// Returns `true` if exit was requested (or will be after killing jobs),
/// `false` if there were background jobs and a warning was shown.
pub fn request_exit(engine_state: &EngineState, exit_code: i32) -> bool {
    let jobs = engine_state.jobs.lock().expect("failed to lock job table");

    if engine_state.is_interactive
        && jobs.iter().next().is_some()
        && !engine_state.exit_warning_given.load(Ordering::SeqCst)
    {
        let job_count = jobs.iter().count();

        println!("There are still background jobs running ({job_count}).");
        println!("Running `exit` a second time will kill all of them.");

        engine_state
            .exit_warning_given
            .store(true, Ordering::SeqCst);

        return false;
    }

    // Request the exit - the REPL will handle it after command execution
    engine_state.request_exit(exit_code);
    true
}

/// Exit the process or clean jobs if appropriate.
///
/// Drops `tag` and exits the current process if there are no running jobs, or if `exit_warning_given` is true.
/// When running in an interactive session, warns the user if there
/// were jobs and sets `exit_warning_given` instead, returning `tag` itself in that case.
///
// Currently, this `tag` argument exists mostly so that a LineEditor can be dropped before exiting the process.
pub fn cleanup_exit<T>(tag: T, engine_state: &EngineState, exit_code: i32) -> T {
    let mut jobs = engine_state.jobs.lock().expect("failed to lock job table");

    if engine_state.is_interactive
        && jobs.iter().next().is_some()
        && !engine_state.exit_warning_given.load(Ordering::SeqCst)
    {
        let job_count = jobs.iter().count();

        println!("There are still background jobs running ({job_count}).");

        println!("Running `exit` a second time will kill all of them.");

        engine_state
            .exit_warning_given
            .store(true, Ordering::SeqCst);

        return tag;
    }

    let _ = jobs.kill_all();

    drop(tag);

    std::process::exit(exit_code);
}
