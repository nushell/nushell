mod collect;
mod command;
mod filter;
mod to_lazy;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::command::LazyDataframe;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::to_lazy::ToLazyFrame;

pub fn add_lazy_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Dataframe commands
    bind_command!(LazyDataframe, ToLazyFrame, LazyCollect, LazyFilter);
}
