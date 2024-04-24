mod alias;
mod arg_where;
mod col;
mod concat_str;
mod datepart;
mod expressions_macro;
mod is_in;
mod lit;
mod otherwise;
mod when;

use nu_plugin::PluginCommand;

pub use crate::dataframe::expressions::alias::ExprAlias;
pub use crate::dataframe::expressions::arg_where::ExprArgWhere;
pub use crate::dataframe::expressions::col::ExprCol;
pub use crate::dataframe::expressions::concat_str::ExprConcatStr;
pub use crate::dataframe::expressions::datepart::ExprDatePart;
pub use crate::dataframe::expressions::expressions_macro::*;
pub use crate::dataframe::expressions::is_in::ExprIsIn;
pub use crate::dataframe::expressions::lit::ExprLit;
pub use crate::dataframe::expressions::otherwise::ExprOtherwise;
pub use crate::dataframe::expressions::when::ExprWhen;
use crate::PolarsPlugin;

pub(crate) fn expr_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ExprAlias),
        Box::new(ExprArgWhere),
        Box::new(ExprAggGroups),
        Box::new(ExprCol),
        Box::new(ExprConcatStr),
        Box::new(ExprCount),
        Box::new(ExprDatePart),
        Box::new(ExprIsIn),
        Box::new(ExprList),
        Box::new(ExprLit),
        Box::new(ExprNot),
        Box::new(ExprMax),
        Box::new(ExprMin),
        Box::new(ExprOtherwise),
        Box::new(ExprSum),
        Box::new(ExprMean),
        Box::new(ExprStd),
        Box::new(ExprVar),
        Box::new(ExprWhen),
    ]
}
