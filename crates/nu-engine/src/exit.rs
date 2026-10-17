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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Signals;
    use nu_protocol::engine::Job;
    use nu_protocol::engine::ThreadJob;
    use rstest::rstest;
    use std::ops::Not;
    use std::sync::mpsc;

    #[rstest]
    #[case(&[true])]
    #[case(&[false])]
    #[case(&[true, false])]
    #[case(&[false, false, true])]
    fn interactive_cleanup_respects_auto_kill(#[case] auto_kill_flags: &[bool]) {
        let mut engine_state = EngineState::new();
        engine_state.is_interactive = true;

        let mut signals_handles = vec![];

        for auto_kill in auto_kill_flags {
            let signals_handle = Signals::new(Default::default());
            signals_handles.push(signals_handle.clone());

            let (send, _) = mpsc::channel();

            let job = ThreadJob::new(signals_handle, None, *auto_kill, send);
            let job = Job::Thread(job);
            engine_state.jobs.lock().unwrap().add_job(job);
        }

        let should_warn = auto_kill_flags.contains(&false);

        let exited = cleanup((), &engine_state).is_none();
        assert_eq!(exited, !should_warn);

        let warned = engine_state.exit_warning_given.load(Ordering::SeqCst);
        assert_eq!(warned, should_warn);

        if should_warn {
            assert_eq!(
                engine_state.jobs.lock().unwrap().iter().count(),
                signals_handles.len()
            );

            assert!(
                signals_handles
                    .iter()
                    .all(|signals| signals.interrupted().not())
            );

            let exited = cleanup((), &engine_state).is_none();
            assert!(exited);
        }

        assert!(engine_state.jobs.lock().unwrap().is_empty());

        assert!(signals_handles.iter().all(Signals::interrupted));
    }
}
