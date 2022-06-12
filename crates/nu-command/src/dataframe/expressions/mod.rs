mod alias;
mod as_nu;
mod col;
mod expressions_macro;
mod lit;
mod otherwise;
mod when;
mod quantile;

use nu_protocol::engine::StateWorkingSet;

pub(crate) use crate::dataframe::expressions::alias::ExprAlias;
use crate::dataframe::expressions::as_nu::ExprAsNu;
pub(super) use crate::dataframe::expressions::col::ExprCol;
pub(crate) use crate::dataframe::expressions::expressions_macro::*;
pub(super) use crate::dataframe::expressions::lit::ExprLit;
pub(super) use crate::dataframe::expressions::otherwise::ExprOtherwise;
pub(super) use crate::dataframe::expressions::when::ExprWhen;
pub(super) use crate::dataframe::expressions::quantile::ExprQuantile;

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
        ExprQuantile,
        ExprList,
        ExprAggGroups,
        ExprFlatten,
        ExprExplode,
        ExprCount,
        ExprFirst,
        ExprLast,
        ExprNUnique,
        ExprIsNotNull,
        ExprIsNull,
        ExprNot,
        ExprMax,
        ExprMin,
        ExprSum,
        ExprMean,
        ExprMedian,
        ExprStd,
        ExprVar
    );
}
