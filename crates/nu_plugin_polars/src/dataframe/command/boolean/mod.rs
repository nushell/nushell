mod all_false;
mod all_true;
mod arg_true;
mod expr_not;
mod is_duplicated;
mod is_in;
mod is_not_null;
mod is_null;
mod is_unique;
mod not;
pub(crate) mod otherwise;
mod set;
mod when;

use crate::PolarsPlugin;
use nu_plugin::PluginCommand;

pub use all_false::AllFalse;
pub use arg_true::ArgTrue;
pub use is_duplicated::IsDuplicated;
pub use is_in::ExprIsIn;
pub use is_not_null::IsNotNull;
pub use is_null::IsNull;
pub use is_unique::IsUnique;
pub use not::NotSeries;
pub use otherwise::ExprOtherwise;
pub use set::SetSeries;
pub use when::ExprWhen;

pub(crate) fn boolean_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AllFalse),
        Box::new(all_true::AllTrue),
        Box::new(ArgTrue),
        Box::new(ExprIsIn),
        Box::new(ExprOtherwise),
        Box::new(ExprWhen),
        Box::new(expr_not::ExprNot),
        Box::new(IsDuplicated),
        Box::new(IsNotNull),
        Box::new(IsNull),
        Box::new(IsUnique),
        Box::new(NotSeries),
        Box::new(SetSeries),
    ]
}
