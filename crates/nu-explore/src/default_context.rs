use nu_protocol::engine::{EngineState, StateWorkingSet};

use crate::explore::*;

pub fn add_explore_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(Explore));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating explore command context: {err:?}");
    }

    engine_state
}
