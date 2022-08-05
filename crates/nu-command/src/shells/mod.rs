mod enter;
mod exit;
mod g;
mod n;
mod p;
mod shells_;

pub use enter::Enter;
pub use exit::Exit;
pub use g::GotoShell;
pub use n::NextShell;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::Value;
pub use p::PrevShell;
pub use shells_::Shells;

pub fn get_shells(engine_state: &EngineState, stack: &mut Stack, cwd: Value) -> Vec<Value> {
    let shells = stack.get_env_var(engine_state, "NUSHELL_SHELLS");
    let shells = if let Some(v) = shells {
        v.as_list()
            .map(|x| x.to_vec())
            .unwrap_or_else(|_| vec![cwd])
    } else {
        vec![cwd]
    };
    shells
}

pub fn get_current_shell(engine_state: &EngineState, stack: &mut Stack) -> usize {
    let current_shell = stack.get_env_var(engine_state, "NUSHELL_CURRENT_SHELL");
    if let Some(v) = current_shell {
        v.as_integer().unwrap_or_default() as usize
    } else {
        0
    }
}
