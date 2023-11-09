pub mod scatter;
use crate::dataframe::plotting::scatter::ScatterPlot;
use nu_protocol::engine::StateWorkingSet;

pub fn add_plotting_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Dataframe commands
    bind_command!(ScatterPlot);
}
