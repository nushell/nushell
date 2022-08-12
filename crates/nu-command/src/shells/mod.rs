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
use nu_engine::current_dir;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{IntoInterruptiblePipelineData, PipelineData, ShellError, Span, Value};
pub use p::PrevShell;
pub use shells_::Shells;

enum SwitchTo {
    Next,
    Prev,
    Last,
    Nth(usize),
}

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

fn get_last_shell(engine_state: &EngineState, stack: &mut Stack) -> usize {
    let last_shell = stack.get_env_var(engine_state, "NUSHELL_LAST_SHELL");
    if let Some(v) = last_shell {
        v.as_integer().unwrap_or_default() as usize
    } else {
        0
    }
}

fn switch_shell(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
    switch_to: SwitchTo,
) -> Result<PipelineData, ShellError> {
    let cwd = current_dir(engine_state, stack)?;
    let cwd = Value::String {
        val: cwd.to_string_lossy().to_string(),
        span: call.head,
    };

    let shells = get_shells(engine_state, stack, cwd);
    let current_shell = get_current_shell(engine_state, stack);

    let new_shell = match switch_to {
        SwitchTo::Next => {
            let mut new_shell = current_shell + 1;

            if new_shell == shells.len() {
                new_shell = 0;
            }

            new_shell
        }
        SwitchTo::Prev => {
            if current_shell == 0 {
                shells.len() - 1
            } else {
                current_shell - 1
            }
        }
        SwitchTo::Last => get_last_shell(engine_state, stack),
        SwitchTo::Nth(n) => n,
    };

    let new_path = shells
        .get(new_shell)
        .ok_or(ShellError::NotFound(span))?
        .to_owned();

    stack.add_env_var(
        "NUSHELL_SHELLS".into(),
        Value::List {
            vals: shells,
            span: call.head,
        },
    );

    stack.add_env_var(
        "NUSHELL_CURRENT_SHELL".into(),
        Value::Int {
            val: new_shell as i64,
            span: call.head,
        },
    );

    stack.add_env_var(
        "NUSHELL_LAST_SHELL".into(),
        Value::Int {
            val: current_shell as i64,
            span: call.head,
        },
    );

    stack.add_env_var("PWD".into(), new_path);

    Ok(PipelineData::new(call.head))
}

fn list_shells(
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let cwd = current_dir(engine_state, stack)?;
    let cwd = Value::String {
        val: cwd.to_string_lossy().to_string(),
        span,
    };

    let shells = get_shells(engine_state, stack, cwd);
    let current_shell = get_current_shell(engine_state, stack);

    Ok(shells
        .into_iter()
        .enumerate()
        .map(move |(idx, val)| Value::Record {
            cols: vec!["active".to_string(), "path".to_string()],
            vals: vec![
                Value::Bool {
                    val: idx == current_shell,
                    span,
                },
                val,
            ],
            span,
        })
        .into_pipeline_data(None))
}
