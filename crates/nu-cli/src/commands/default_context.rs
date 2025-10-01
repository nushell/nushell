use crate::commands::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn add_cli_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        bind_command! {
            Commandline,
            CommandlineEdit,
            CommandlineGetCursor,
            CommandlineSetCursor,
            History,
            Keybindings,
            KeybindingsDefault,
            KeybindingsList,
            KeybindingsListen,
        };

        #[cfg(feature = "sqlite")]
        bind_command! {
            HistoryImport,
            HistorySession
        };

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating CLI command context: {err:?}");
    }

    engine_state
}
