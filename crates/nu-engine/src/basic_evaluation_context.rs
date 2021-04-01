use crate::Scope;
use crate::{basic_shell_manager, config_holder::ConfigHolder};
use crate::{env::basic_host::BasicHost, Host};
use crate::{filesystem::filesystem_shell::FilesystemShellMode, EvaluationContext};
use indexmap::IndexMap;
use parking_lot::Mutex;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn basic_evaluation_context(
    mode: FilesystemShellMode,
) -> Result<EvaluationContext, Box<dyn Error>> {
    let scope = Scope::new();
    let mut host = BasicHost {};
    let env_vars = host.vars().iter().cloned().collect::<IndexMap<_, _>>();
    scope.add_env(env_vars);

    Ok(EvaluationContext {
        scope,
        host: Arc::new(parking_lot::Mutex::new(Box::new(host))),
        current_errors: Arc::new(Mutex::new(vec![])),
        ctrl_c: Arc::new(AtomicBool::new(false)),
        configs: Arc::new(Mutex::new(ConfigHolder::new())),
        shell_manager: basic_shell_manager::basic_shell_manager(mode)?,
        windows_drives_previous_cwd: Arc::new(Mutex::new(std::collections::HashMap::new())),
    })
}
