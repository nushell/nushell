mod date;
pub use date::*;

mod string;
pub use string::*;

mod indexes;
pub use indexes::*;

mod arg_max;
mod arg_min;
mod cumulative;
mod n_null;
mod n_unique;

use nu_plugin::PluginCommand;

use crate::PolarsPlugin;
pub use arg_max::ArgMax;
pub use arg_min::ArgMin;
pub use cumulative::Cumulative;
pub use n_null::NNull;
pub use n_unique::NUnique;

pub(crate) fn series_commands() -> Vec<Box<dyn PluginCommand<Plugin = PolarsPlugin>>> {
    vec![
        Box::new(ArgMax),
        Box::new(ArgMin),
        Box::new(ArgSort),
        Box::new(ArgUnique),
        Box::new(AsDate),
        Box::new(AsDateTime),
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
        Box::new(NNull),
        Box::new(NUnique),
        Box::new(Replace),
        Box::new(ReplaceAll),
        Box::new(SetWithIndex),
        Box::new(StrJoin),
        Box::new(StrLengths),
        Box::new(StrSlice),
        Box::new(StrFTime),
        Box::new(ToDecimal),
        Box::new(ToInteger),
        Box::new(ToLowerCase),
        Box::new(ToUpperCase),
    ]
}
