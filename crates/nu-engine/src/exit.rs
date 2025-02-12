use std::sync::atomic::Ordering;

use nu_protocol::engine::EngineState;

/// Exiting the process or clean jobs if appropriate.
///
/// Drops `tag` and exits the current process if there are no running jobs, or if `exit_warning_given` is true.
/// When running in an interactive session, warns the user if there
/// were jobs and sets `exit_warning_given` instead, returning `tag` itself in that case.
///
// Currently, this `tag` argument exists mostly so that a LineEditor can be dropped before exiting the process.
pub fn cleanup_exit<T>(tag: T, engine_state: &EngineState, exit_code: i32) -> T {
    if engine_state.is_interactive {
        let jobs = engine_state.jobs.lock().expect("failed to lock job table");

        if jobs.iter().next().is_some() {
            if engine_state.exit_warning_given.load(Ordering::SeqCst) {
                for job in jobs.iter() {
                    let _ = job.1.kill();
                }
            } else {
                let job_count = jobs.iter().count();

                println!("There are still {} background jobs running.", job_count);

                println!("Running `exit` a second time will kill all of them.");

                engine_state
                    .exit_warning_given
                    .store(true, Ordering::SeqCst);

                return tag;
            }
        }
    }

    drop(tag);
    std::process::exit(exit_code);
}
