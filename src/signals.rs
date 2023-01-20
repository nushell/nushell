use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use nu_protocol::engine::EngineState;

pub(crate) fn ctrlc_protection(engine_state: &mut EngineState, ctrlc: &Arc<AtomicBool>) {
    let handler_ctrlc = ctrlc.clone();
    let engine_state_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(engine_state_ctrlc);
}

#[cfg(not(windows))]
pub(crate) fn sigquit_protection(engine_state: &mut EngineState) {
    use signal_hook::consts::SIGQUIT;
    let sig_quit = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGQUIT, sig_quit.clone()).expect("Error setting SIGQUIT flag");
    engine_state.set_sig_quit(sig_quit);
}

#[cfg(windows)]
pub(crate) fn sigquit_protection(_engine_state: &mut EngineState) {}
