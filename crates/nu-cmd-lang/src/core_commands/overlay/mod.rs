mod command;
mod hide;
mod list;
mod new;
mod use_;

pub use command::Overlay;
pub use hide::OverlayHide;
pub use list::OverlayList;
pub use new::OverlayNew;
pub use use_::OverlayUse;

use nu_protocol::engine::{EngineState, Stack};

/// Runs `change`, which adds or removes overlays, then puts `PWD` and `OLDPWD` back.
///
/// Each overlay holds its own copy of both, but switching overlays doesn't move you.
fn keep_cwd(engine_state: &EngineState, stack: &mut Stack, change: impl FnOnce(&mut Stack)) {
    let cwd_vars: Vec<_> = ["PWD", "OLDPWD"]
        .into_iter()
        .filter_map(|name| Some((name, stack.get_env_var(engine_state, name)?.clone())))
        .collect();

    change(stack);

    for (name, value) in cwd_vars {
        stack.add_env_var(name.to_string(), value);
    }
}
