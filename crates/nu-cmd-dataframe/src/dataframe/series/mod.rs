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

use nu_protocol::engine::StateWorkingSet;

pub use all_false::AllFalse;
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

pub fn add_series_decls(working_set: &mut StateWorkingSet) {
    macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

    // Series commands
    bind_command!(
        AllFalse,
        AllTrue,
        ArgMax,
        ArgMin,
        ArgSort,
        ArgTrue,
        ArgUnique,
        AsDate,
        AsDateTime,
        Concatenate,
        Contains,
        Cumulative,
        GetDay,
        GetHour,
        GetMinute,
        GetMonth,
        GetNanosecond,
        GetOrdinal,
        GetSecond,
        GetWeek,
        GetWeekDay,
        GetYear,
        IsDuplicated,
        IsIn,
        IsNotNull,
        IsNull,
        IsUnique,
        NNull,
        NUnique,
        NotSeries,
        Replace,
        ReplaceAll,
        Rolling,
        SetSeries,
        SetWithIndex,
        Shift,
        StrLengths,
        StrSlice,
        StrFTime,
        ToLowerCase,
        ToUpperCase,
        Unique,
        ValueCount
    );
}
