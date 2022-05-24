mod alias;
mod expressions_macro;
mod as_nu;
mod col;
mod lit;
mod when;
mod otherwise;

use nu_protocol::engine::StateWorkingSet;

pub(super) use crate::dataframe::expressions::col::ExprCol;
pub(super) use crate::dataframe::expressions::lit::ExprLit;
pub(super) use crate::dataframe::expressions::when::ExprWhen;
pub(super) use crate::dataframe::expressions::otherwise::ExprOtherwise;
pub(crate) use crate::dataframe::expressions::alias::ExprAlias;
pub(crate) use crate::dataframe::expressions::expressions_macro::*;
use crate::dataframe::expressions::as_nu::ExprAsNu;

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
        ExprCount,
        ExprLit,
        ExprAsNu,
        ExprWhen,
        ExprOtherwise,
        ExprList,
        ExprAggGroups,
        ExprFlatten,
        ExprExplode
    );
}
