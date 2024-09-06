mod alias;
mod arg_where;
mod col;
mod concat_str;
mod datepart;
mod lit;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use crate::dataframe::expressions::alias::ExprAlias;
pub use crate::dataframe::expressions::arg_where::ExprArgWhere;
pub use crate::dataframe::expressions::col::ExprCol;
pub use crate::dataframe::expressions::concat_str::ExprConcatStr;
pub use crate::dataframe::expressions::datepart::ExprDatePart;
pub use crate::dataframe::expressions::lit::ExprLit;

pub(crate) fn expr_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ExprAlias),
        Box::new(ExprArgWhere),
        Box::new(ExprCol),
        Box::new(ExprConcatStr),
        Box::new(ExprDatePart),
        Box::new(ExprLit),
    ]
}
