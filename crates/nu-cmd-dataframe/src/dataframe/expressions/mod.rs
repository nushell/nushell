mod alias;
mod arg_where;
mod col;
mod concat_str;
mod datepart;
mod expressions_macro;
mod is_in;
mod lit;
mod otherwise;
mod quantile;
mod when;

use nu_protocol::engine::StateWorkingSet;

use crate::dataframe::expressions::arg_where::ExprArgWhere;
pub(crate) use crate::dataframe::expressions::{
    alias::ExprAlias, datepart::ExprDatePart, expressions_macro::*,
};
pub(super) use crate::dataframe::expressions::{
    col::ExprCol, concat_str::ExprConcatStr, is_in::ExprIsIn, lit::ExprLit,
    otherwise::ExprOtherwise, quantile::ExprQuantile, when::ExprWhen,
};

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
        ExprWhen,
        ExprOtherwise,
        ExprQuantile,
        ExprList,
        ExprAggGroups,
        ExprCount,
        ExprIsIn,
        ExprNot,
        ExprMax,
        ExprMin,
        ExprSum,
        ExprMean,
        ExprMedian,
        ExprStd,
        ExprVar,
        ExprDatePart
    );
}
