mod eager;
mod expressions;
mod lazy;
mod series;
mod stub;
mod utils;
mod values;

pub use eager::add_eager_decls;
pub use expressions::add_expressions;
pub use lazy::add_lazy_decls;
pub use series::add_series_decls;

use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn add_dataframe_context(mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(stub::Dfr));
        add_series_decls(&mut working_set);
        add_eager_decls(&mut working_set);
        add_expressions(&mut working_set);
        add_lazy_decls(&mut working_set);

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating dataframe command context: {err:?}");
    }

    engine_state
}

#[cfg(test)]
mod test_dataframe;
