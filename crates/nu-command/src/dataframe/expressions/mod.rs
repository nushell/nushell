mod alias;
mod arg_where;
mod as_nu;
mod col;
mod concat_str;
mod expressions_macro;
mod is_in;
mod lit;
mod otherwise;
mod quantile;
mod when;

use nu_protocol::engine::StateWorkingSet;

pub(crate) use crate::dataframe::expressions::alias::ExprAlias;
use crate::dataframe::expressions::arg_where::ExprArgWhere;
use crate::dataframe::expressions::as_nu::ExprAsNu;
pub(super) use crate::dataframe::expressions::col::ExprCol;
pub(super) use crate::dataframe::expressions::concat_str::ExprConcatStr;
pub(crate) use crate::dataframe::expressions::expressions_macro::*;
pub(super) use crate::dataframe::expressions::is_in::ExprIsIn;
pub(super) use crate::dataframe::expressions::lit::ExprLit;
pub(super) use crate::dataframe::expressions::otherwise::ExprOtherwise;
pub(super) use crate::dataframe::expressions::quantile::ExprQuantile;
pub(super) use crate::dataframe::expressions::when::ExprWhen;

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
        ExprArgWhere,
        ExprCol,
        ExprConcatStr,
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
        ExprIsIn,
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
