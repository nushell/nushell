mod charting;
mod config;
mod conversions;
mod core_commands;
#[cfg(feature = "dataframe")]
mod dataframe;
mod env;
mod filesystem;
mod filters;
mod formats;
mod generators;
mod math;
mod network;
mod path;
mod platform;
mod random;
mod shells;
mod strings;
mod viewers;

pub use charting::*;
pub use config::*;
pub use conversions::*;
pub use core_commands::*;
#[cfg(feature = "dataframe")]
pub use dataframe::{
    DataFrame, DataFrameAggregate, DataFrameAllFalse, DataFrameAllTrue, DataFrameArgMax,
    DataFrameArgMin, DataFrameArgSort, DataFrameArgTrue, DataFrameArgUnique, DataFrameColumn,
    DataFrameDTypes, DataFrameDrop, DataFrameDropDuplicates, DataFrameDropNulls, DataFrameDummies,
    DataFrameFilter, DataFrameGet, DataFrameGroupBy, DataFrameHead, DataFrameIsDuplicated,
    DataFrameIsIn, DataFrameIsNotNull, DataFrameIsNull, DataFrameIsUnique, DataFrameJoin,
    DataFrameList, DataFrameLoad, DataFrameMelt, DataFrameNNull, DataFrameNUnique, DataFramePivot,
    DataFrameSample, DataFrameSelect, DataFrameSeriesRename, DataFrameSet, DataFrameShift,
    DataFrameShow, DataFrameSlice, DataFrameSort, DataFrameTail, DataFrameToCsv, DataFrameToDF,
    DataFrameToParquet, DataFrameToSeries, DataFrameUnique, DataFrameValueCounts, DataFrameWhere,
    DataFrameWithColumn,
};
pub use env::*;
pub use filesystem::*;
pub use filters::*;
pub use formats::*;
pub use generators::*;
pub use math::*;
pub use network::*;
pub use path::*;
pub use platform::*;
pub use random::*;
pub use shells::*;
pub use strings::*;
pub use viewers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::{test_anchors, test_examples};
    use nu_engine::{whole_stream_command, Command};
    use nu_errors::ShellError;

    fn full_tests() -> Vec<Command> {
        vec![
            whole_stream_command(Append),
            whole_stream_command(GroupBy),
            whole_stream_command(Insert),
            whole_stream_command(MoveColumn),
            whole_stream_command(Update),
            whole_stream_command(Empty),
            // whole_stream_command(Select),
            // whole_stream_command(Get),
            // Str Command Suite
            whole_stream_command(Str),
            whole_stream_command(StrToDecimal),
            whole_stream_command(StrToInteger),
            whole_stream_command(StrDowncase),
            whole_stream_command(StrUpcase),
            whole_stream_command(StrCapitalize),
            whole_stream_command(StrFindReplace),
            whole_stream_command(StrSubstring),
            whole_stream_command(StrToDatetime),
            whole_stream_command(StrContains),
            whole_stream_command(StrIndexOf),
            whole_stream_command(StrTrim),
            whole_stream_command(StrTrimLeft),
            whole_stream_command(StrTrimRight),
            whole_stream_command(StrStartsWith),
            whole_stream_command(StrEndsWith),
            //whole_stream_command(StrCollect),
            whole_stream_command(StrLength),
            whole_stream_command(StrLPad),
            whole_stream_command(StrReverse),
            whole_stream_command(StrRPad),
            whole_stream_command(StrCamelCase),
            whole_stream_command(StrPascalCase),
            whole_stream_command(StrKebabCase),
            whole_stream_command(StrSnakeCase),
            whole_stream_command(StrScreamingSnakeCase),
            whole_stream_command(ToMarkdown),
        ]
    }

    fn only_examples() -> Vec<Command> {
        let mut commands = full_tests();
        commands.extend(vec![whole_stream_command(Flatten)]);
        commands
    }

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        for cmd in only_examples() {
            println!("cmd: {}", cmd.name());
            test_examples(cmd)?;
        }

        Ok(())
    }

    #[test]
    fn tracks_metadata() -> Result<(), ShellError> {
        for cmd in full_tests() {
            test_anchors(cmd)?;
        }

        Ok(())
    }
}
