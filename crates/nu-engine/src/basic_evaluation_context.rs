use crate::env::basic_host::BasicHost;
use crate::EvaluationContext;
use crate::Scope;
use crate::{basic_shell_manager, config_holder::ConfigHolder};
use parking_lot::Mutex;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn basic_evaluation_context() -> Result<EvaluationContext, Box<dyn Error>> {
    Ok(EvaluationContext {
        scope: Scope::new(),
        host: Arc::new(parking_lot::Mutex::new(Box::new(BasicHost))),
        current_errors: Arc::new(Mutex::new(vec![])),
        ctrl_c: Arc::new(AtomicBool::new(false)),
        configs: Arc::new(Mutex::new(ConfigHolder::new())),
        shell_manager: basic_shell_manager::basic_shell_manager()?,
        windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
    })
}
