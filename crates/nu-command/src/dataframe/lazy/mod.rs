mod into_expression;

mod collect;
mod command;
mod filter;
mod groupby;
mod aggregate;
mod reverse;
mod to_lazy;
mod with_column;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::lazy::collect::LazyCollect;
use crate::dataframe::lazy::command::LazyDataframe;
use crate::dataframe::lazy::filter::LazyFilter;
use crate::dataframe::lazy::groupby::ToLazyGroupBy;
use crate::dataframe::lazy::aggregate::LazyAggregate;
use crate::dataframe::lazy::reverse::LazyReverse;
use crate::dataframe::lazy::to_lazy::ToLazyFrame;
use crate::dataframe::lazy::with_column::LazyWithColumn;

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
    bind_command!(
        LazyAggregate,
        LazyDataframe,
        LazyCollect,
        LazyFilter,
        LazyReverse,
        LazyWithColumn,
        ToLazyFrame,
        ToLazyGroupBy
    );
}
