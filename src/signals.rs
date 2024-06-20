use nu_protocol::engine::EngineState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};

pub(crate) fn ctrlc_protection(
    engine_state: &mut EngineState,
    ctrlc: &Arc<AtomicBool>,
    subscribers: &Arc<Mutex<Vec<mpsc::Sender<()>>>>,
) {
    {
        let ctrlc = ctrlc.clone();
        let subscribers = subscribers.clone();

        ctrlc::set_handler(move || {
            ctrlc.store(true, Ordering::SeqCst);
            if let Ok(subscribers) = subscribers.lock() {
                for subscriber in subscribers.iter() {
                    let _ = subscriber.send(());
                }
            }
        })
        .expect("Error setting Ctrl-C handler");
    }

    engine_state.ctrlc = Some(ctrlc.clone());
    engine_state.ctrlc_tx = Some(subscribers.clone());
}
