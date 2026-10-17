use nu_engine::exit::kill_all_jobs;
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
/// Outside the REPL, Ctrl-C kills background jobs before publishing the interrupt: commands and
/// scripts, even with `-i`, never return to a prompt where the jobs could be managed. The REPL
/// keeps its background and frozen jobs when the prompt or a foreground command is interrupted.
pub(crate) fn ctrlc_protection(engine_state: &mut EngineState, is_repl: bool) {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();
    engine_state.signal_handlers = Some(signal_handlers.clone());
    let jobs = engine_state.jobs.clone();

    ctrlc::set_handler(move || {
        handle_interrupt(&interrupt, &signal_handlers, &jobs, is_repl);
    })
    .expect("Error setting Ctrl-C handler");
}

fn handle_interrupt(
    interrupt: &AtomicBool,
    signal_handlers: &Handlers,
    jobs: &Mutex<Jobs>,
    is_repl: bool,
) {
    if !is_repl {
        // Finish cleanup before evaluation can catch the interrupt and spawn new work.
        kill_all_jobs(jobs);
    }
    interrupt.store(true, Ordering::Relaxed);
    signal_handlers.run(SignalAction::Interrupt);
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
        let job = ThreadJob::new(job_signals.clone(), None, false, sender);
        let id = jobs.lock().unwrap().add_job(Job::Thread(job));

        (jobs, id, job_signals)
    }

    #[test]
    fn interrupt_kills_background_jobs_before_notifying_handlers() {
        let (jobs, id, job_signals) = jobs_with_background_job();
        let interrupt = Arc::new(AtomicBool::new(false));
        let signal_handlers = Handlers::new();
        let handler_jobs = jobs.clone();
        let handler_interrupt = interrupt.clone();
        signal_handlers
            .register_unguarded(Box::new(move |action| {
                assert!(matches!(action, SignalAction::Interrupt));
                assert!(handler_interrupt.load(Ordering::Relaxed));
                assert!(handler_jobs.lock().unwrap().is_empty());
            }))
            .unwrap();

        handle_interrupt(&interrupt, &signal_handlers, &jobs, false);

        assert!(interrupt.load(Ordering::Relaxed));
        assert!(job_signals.interrupted());
        assert!(jobs.lock().unwrap().lookup(id).is_none());
    }

    #[test]
    fn repl_interrupt_keeps_background_jobs() {
        let (jobs, id, job_signals) = jobs_with_background_job();
        let interrupt = AtomicBool::new(false);

        handle_interrupt(&interrupt, &Handlers::new(), &jobs, true);

        assert!(interrupt.load(Ordering::Relaxed));
        assert!(!job_signals.interrupted());
        assert!(jobs.lock().unwrap().lookup(id).is_some());
    }

    #[test]
    fn interrupt_cleans_poisoned_job_table() {
        let (jobs, _, job_signals) = jobs_with_background_job();
        let poisoned = jobs.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.lock().unwrap();
            panic!("poison the job table");
        })
        .join();
        let interrupt = AtomicBool::new(false);

        handle_interrupt(&interrupt, &Handlers::new(), &jobs, false);

        assert!(interrupt.load(Ordering::Relaxed));
        assert!(job_signals.interrupted());
        assert!(jobs.lock().unwrap_err().into_inner().is_empty());
    }
}
