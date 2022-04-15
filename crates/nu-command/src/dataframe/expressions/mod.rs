mod alias;
mod dsl;
mod to_nu;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::expressions::dsl::*;

use crate::dataframe::expressions::alias::ExprAlias;
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
    bind_command!(
        ExprAlias,
        ExprCol,
        ExprLit,
        ExprToNu,
        ExprWhen
    );
}
