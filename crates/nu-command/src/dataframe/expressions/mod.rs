mod command;
mod dsl;
mod gt;
mod to_nu;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::expressions::dsl::*;

use crate::dataframe::expressions::command::LazyExpression;
use crate::dataframe::expressions::gt::ExprGt;
use crate::dataframe::expressions::to_nu::ExprToNu;

pub fn add_expressions(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Dataframe commands
    bind_command!(LazyExpression, ExprCol, ExprGt, ExprLit, ExprToNu);
}
