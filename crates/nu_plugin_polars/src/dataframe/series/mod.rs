mod date;
pub use date::*;

mod string;
pub use string::*;

mod masks;
pub use masks::*;

mod indexes;
pub use indexes::*;

mod all_false;
mod all_true;
mod arg_max;
mod arg_min;
mod cumulative;
mod n_null;
mod n_unique;
mod rolling;
mod shift;
mod unique;
mod value_counts;

pub use all_false::AllFalse;
use nu_plugin::PluginCommand;

use crate::PolarsPlugin;
pub use all_true::AllTrue;
pub use arg_max::ArgMax;
pub use arg_min::ArgMin;
pub use cumulative::Cumulative;
pub use n_null::NNull;
pub use n_unique::NUnique;
pub use rolling::Rolling;
pub use shift::Shift;
pub use unique::Unique;
pub use value_counts::ValueCount;

pub(crate) fn series_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(AllFalse),
        Box::new(AllTrue),
        Box::new(ArgMax),
        Box::new(ArgMin),
        Box::new(ArgSort),
        Box::new(ArgTrue),
        Box::new(ArgUnique),
        Box::new(AsDate),
        Box::new(AsDateTime),
        Box::new(Concatenate),
        Box::new(Contains),
        Box::new(Cumulative),
        Box::new(GetDay),
        Box::new(GetHour),
        Box::new(GetMinute),
        Box::new(GetMonth),
        Box::new(GetNanosecond),
        Box::new(GetOrdinal),
        Box::new(GetSecond),
        Box::new(GetWeek),
        Box::new(GetWeekDay),
        Box::new(GetYear),
        Box::new(IsDuplicated),
        Box::new(IsNotNull),
        Box::new(IsNull),
        Box::new(IsUnique),
        Box::new(NNull),
        Box::new(NUnique),
        Box::new(NotSeries),
        Box::new(Replace),
        Box::new(ReplaceAll),
        Box::new(Rolling),
        Box::new(SetSeries),
        Box::new(SetWithIndex),
        Box::new(Shift),
        Box::new(StrLengths),
        Box::new(StrSlice),
        Box::new(StrFTime),
        Box::new(ToLowerCase),
        Box::new(ToUpperCase),
        Box::new(Unique),
        Box::new(ValueCount),
    ]
}
