use crate::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn add_plugin_command_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        bind_command!(
            PluginAdd,
            PluginCommand,
            PluginList,
            PluginRm,
            PluginStop,
            PluginUse,
        );

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    engine_state
}
