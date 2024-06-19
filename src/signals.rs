use nu_protocol::engine::EngineState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub(crate) fn ctrlc_protection(
    engine_state: &mut EngineState,
    ctrlc: &Arc<AtomicBool>,
    tx: &Arc<Mutex<bus::Bus<()>>>,
) {
    let handler_ctrlc = ctrlc.clone();
    let handler_tx = tx.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
        if let Ok(mut bus) = handler_tx.lock() {
            let _ = bus.try_broadcast(());
        }
    })
    .expect("Error setting Ctrl-C handler");

    engine_state.ctrlc = Some(ctrlc.clone());
    engine_state.ctrlc_bus = Some(tx.clone());
}
