use crate::commands::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

/// Register the `tui` command family.
pub fn add_tui_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }
        bind_command! {
            Tui,
            TuiRun,
            TuiDebug,
            TuiBind,
            TuiLabel,
            TuiMenu,
            TuiTextBox,
            TuiTable,
            TuiSelect,
            TuiButton,
            TuiProgress,
            TuiLog,
            TuiTree,
            TuiTab,
            TuiBox,
            TuiSplit,
            TuiSearch,
            TuiPreview,
        };
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating tui command context: {err:?}");
    }

    engine_state
}
