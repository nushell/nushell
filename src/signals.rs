use nu_protocol::{Handlers, SignalAction, Signals, engine::EngineState};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState) {
    ctrlc::set_handler(ctrlc_handler(engine_state)).expect("Error setting Ctrl-C handler");
}

fn ctrlc_handler(engine_state: &mut EngineState) -> impl FnMut() + Send + 'static {
    let interrupt = Arc::new(AtomicBool::new(false));
    engine_state.set_signals(Signals::new(interrupt.clone()));

    let signal_handlers = Handlers::new();

    // Non-interactive execution must clean up background jobs on interrupt.
    // In the REPL, interrupting foreground work (including the prompt) must leave
    // background and frozen jobs available for later use.
    if !engine_state.is_interactive {
        signal_handlers
            .register_unguarded({
                let jobs = engine_state.jobs.clone();
                Box::new(move |action| {
                    if action == SignalAction::Interrupt
                        && let Ok(mut jobs) = jobs.lock()
                    {
                        let _ = jobs.kill_all();
                    }
                })
            })
            .expect("Failed to register interrupt signal handler");
    }

    engine_state.signal_handlers = Some(signal_handlers.clone());

    move || {
        interrupt.store(true, Ordering::Relaxed);
        signal_handlers.run(SignalAction::Interrupt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{
        JobId,
        engine::{Job, ThreadJob},
    };
    use std::sync::mpsc;

    fn engine_with_background_job(interactive: bool) -> (EngineState, JobId, Signals) {
        let mut engine_state = EngineState::new();
        engine_state.is_interactive = interactive;

        let job_signals = Signals::new(Arc::new(AtomicBool::new(false)));
        let (sender, _receiver) = mpsc::channel();
        let job = ThreadJob::new(job_signals.clone(), None, sender);
        let id = engine_state.jobs.lock().unwrap().add_job(Job::Thread(job));

        (engine_state, id, job_signals)
    }

    #[test]
    fn interactive_interrupt_preserves_background_jobs() {
        let (mut engine_state, id, job_signals) = engine_with_background_job(true);

        ctrlc_handler(&mut engine_state)();

        assert!(engine_state.signals().interrupted());
        assert!(!job_signals.interrupted());
        assert!(engine_state.jobs.lock().unwrap().lookup(id).is_some());
    }

    #[test]
    fn non_interactive_interrupt_kills_background_jobs() {
        let (mut engine_state, id, job_signals) = engine_with_background_job(false);

        ctrlc_handler(&mut engine_state)();

        assert!(engine_state.signals().interrupted());
        assert!(job_signals.interrupted());
        assert!(engine_state.jobs.lock().unwrap().lookup(id).is_none());
    }
}
