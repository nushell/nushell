use nu_protocol::{
    Handlers, SignalAction, Signals,
    engine::{EngineState, Jobs},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// Install the Ctrl-C handler, along with the engine's [`Signals`] and signal [`Handlers`] it
/// drives.
///
/// Ctrl-C sets the interrupt flag and runs the handlers. Outside the REPL, it also kills all
/// background jobs: commands and scripts, even with `-i`, never return to a prompt where the jobs
/// could be managed. The REPL keeps its background and frozen jobs when the prompt or a
/// foreground command is interrupted.
pub(crate) fn ctrlc_protection(engine_state: &mut EngineState, is_repl: bool) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();
    if !is_repl {
        kill_jobs_on_interrupt(&signal_handlers, engine_state.jobs.clone());
    }
    engine_state.signal_handlers = Some(signal_handlers.clone());

    ctrlc::set_handler(move || {
        interrupt.store(true, Ordering::Relaxed);
        signal_handlers.run(SignalAction::Interrupt);
    })
    .expect("Error setting Ctrl-C handler");
}

/// Register a handler that kills all `jobs` when `signal_handlers` run on interrupt.
fn kill_jobs_on_interrupt(signal_handlers: &Handlers, jobs: Arc<Mutex<Jobs>>) {
    signal_handlers
        .register_unguarded(Box::new(move |action| {
            if action == SignalAction::Interrupt
                && let Ok(mut jobs) = jobs.lock()
            {
                let _ = jobs.kill_all();
            }
        }))
        .expect("Failed to register interrupt signal handler");
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{
        JobId,
        engine::{Job, ThreadJob},
    };
    use std::sync::mpsc;

    fn jobs_with_background_job() -> (Arc<Mutex<Jobs>>, JobId, Signals) {
        let jobs = Arc::new(Mutex::new(Jobs::default()));
        let job_signals = Signals::new(Arc::new(AtomicBool::new(false)));
        let (sender, _receiver) = mpsc::channel();
        let job = ThreadJob::new(job_signals.clone(), None, sender);
        let id = jobs.lock().unwrap().add_job(Job::Thread(job));

        (jobs, id, job_signals)
    }

    #[test]
    fn interrupt_kills_background_jobs() {
        let (jobs, id, job_signals) = jobs_with_background_job();
        let signal_handlers = Handlers::new();
        kill_jobs_on_interrupt(&signal_handlers, jobs.clone());

        signal_handlers.run(SignalAction::Interrupt);

        assert!(job_signals.interrupted());
        assert!(jobs.lock().unwrap().lookup(id).is_none());
    }

    #[test]
    fn reset_keeps_background_jobs() {
        let (jobs, id, job_signals) = jobs_with_background_job();
        let signal_handlers = Handlers::new();
        kill_jobs_on_interrupt(&signal_handlers, jobs.clone());

        signal_handlers.run(SignalAction::Reset);

        assert!(!job_signals.interrupted());
        assert!(jobs.lock().unwrap().lookup(id).is_some());
    }
}
