use std::sync::{Mutex, PoisonError, atomic::Ordering};

use nu_protocol::engine::{EngineState, Jobs};

/// Exit the process or clean jobs if appropriate.
///
/// Drops `tag` and exits the current process if there are no running jobs, or if `exit_warning_given` is true.
/// When running in a REPL session, warns the user if there
/// were jobs and sets `exit_warning_given` instead, returning `tag` itself in that case.
///
// Currently, this `tag` argument exists mostly so that a LineEditor can be dropped before exiting the process.
pub fn cleanup_exit<T>(tag: T, engine_state: &EngineState, exit_code: i32) -> T {
    if let Some(tag) = cleanup(tag, engine_state) {
        return tag;
    }

    std::process::exit(exit_code);
}

/// Clean up jobs before exiting, unless the REPL needs to warn about running jobs.
///
/// On the first REPL exit with running jobs, sets `exit_warning_given` and returns
/// `Some(tag)` so the caller can return to the prompt. Otherwise, kills the jobs, drops
/// `tag`, and returns `None` so the caller can exit.
pub fn cleanup<T>(tag: T, engine_state: &EngineState) -> Option<T> {
    let jobs = engine_state
        .jobs
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    if engine_state.is_repl
        && jobs.iter().next().is_some()
        && !engine_state.exit_warning_given.load(Ordering::SeqCst)
    {
        let job_count = jobs.iter().count();

        println!("There are still background jobs running ({job_count}).");

        println!("Running `exit` a second time will kill all of them.");

        engine_state
            .exit_warning_given
            .store(true, Ordering::SeqCst);

        return Some(tag);
    }

    drop(jobs);
    kill_all_jobs(&engine_state.jobs);

    drop(tag);
    None
}

/// Kill all background jobs, then exit the current process.
///
/// Unlike [`cleanup_exit`], which in a REPL session warns about running jobs and keeps Nu
/// running instead, this always kills them and exits. Use it where Nu exits without returning to
/// a prompt, such as when commands (`-c`) or a script finish or fail, even with `-i`: jobs still
/// in the table would outlive Nu.
pub fn kill_jobs_and_exit(engine_state: &EngineState, exit_code: i32) -> ! {
    kill_all_jobs(&engine_state.jobs);
    std::process::exit(exit_code)
}

/// Kill all background jobs and remove them from the job table, without warning.
///
/// For when Nu exits without a prompt to warn at; see [`kill_jobs_and_exit`]. A poisoned job
/// table is still cleaned up, since its jobs would otherwise outlive Nu.
pub fn kill_all_jobs(jobs: &Mutex<Jobs>) {
    let mut jobs = jobs.lock().unwrap_or_else(PoisonError::into_inner);
    let _ = jobs.kill_all();
}
