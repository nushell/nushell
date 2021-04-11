#[macro_use]
pub(crate) mod macros;

mod from_delimited_data;
mod to_delimited_data;

pub(crate) mod all;
pub(crate) mod ansi;
pub(crate) mod any;
pub(crate) mod append;
pub(crate) mod args;
pub mod autoenv;
pub(crate) mod autoenv_trust;
pub(crate) mod autoenv_untrust;
pub(crate) mod autoview;
pub(crate) mod benchmark;
pub(crate) mod build_string;
pub(crate) mod cal;
pub(crate) mod cd;
pub(crate) mod char_;
pub(crate) mod chart;
pub(crate) mod classified;
#[cfg(feature = "clipboard-cli")]
pub(crate) mod clip;
pub(crate) mod compact;
pub(crate) mod config;
pub(crate) mod constants;
pub(crate) mod cp;
pub(crate) mod date;
pub(crate) mod debug;
pub(crate) mod def;
pub(crate) mod default;
pub mod default_context;
pub(crate) mod describe;
pub(crate) mod do_;
pub(crate) mod drop;
pub(crate) mod du;
pub(crate) mod each;
pub(crate) mod echo;
pub(crate) mod empty;
pub(crate) mod enter;
pub(crate) mod every;
pub(crate) mod exec;
pub(crate) mod exit;
pub(crate) mod first;
pub(crate) mod flatten;
pub(crate) mod format;
pub(crate) mod from;
pub(crate) mod from_csv;
pub(crate) mod from_eml;
pub(crate) mod from_ics;
pub(crate) mod from_ini;
pub(crate) mod from_json;
pub(crate) mod from_ods;
pub(crate) mod from_ssv;
pub(crate) mod from_toml;
pub(crate) mod from_tsv;
pub(crate) mod from_url;
pub(crate) mod from_vcf;
pub(crate) mod from_xlsx;
pub(crate) mod from_xml;
pub(crate) mod from_yaml;
pub(crate) mod get;
pub(crate) mod group_by;
pub(crate) mod group_by_date;
pub(crate) mod hash_;
pub(crate) mod headers;
pub(crate) mod help;
pub(crate) mod histogram;
pub(crate) mod history;
pub(crate) mod if_;
pub(crate) mod insert;
pub(crate) mod into;
pub(crate) mod keep;
pub(crate) mod last;
pub(crate) mod length;
pub(crate) mod let_;
pub(crate) mod let_env;
pub(crate) mod lines;
pub(crate) mod ls;
pub(crate) mod math;
pub(crate) mod merge;
pub(crate) mod mkdir;
pub(crate) mod move_;
pub(crate) mod next;
pub(crate) mod nth;
pub(crate) mod nu;
pub(crate) mod open;
pub(crate) mod parse;
pub(crate) mod path;
pub(crate) mod pivot;
pub(crate) mod prepend;
pub(crate) mod prev;
pub(crate) mod pwd;
pub(crate) mod random;
pub(crate) mod range;
pub(crate) mod reduce;
pub(crate) mod reject;
pub(crate) mod rename;
pub(crate) mod reverse;
pub(crate) mod rm;
pub(crate) mod roll;
pub(crate) mod rotate;
pub(crate) mod run_external;
pub(crate) mod save;
pub(crate) mod select;
pub(crate) mod seq;
pub(crate) mod seq_dates;
pub(crate) mod shells;
pub(crate) mod shuffle;
pub(crate) mod size;
pub(crate) mod skip;
pub(crate) mod sleep;
pub(crate) mod sort_by;
pub(crate) mod source;
pub(crate) mod split;
pub(crate) mod split_by;
pub(crate) mod str_;
pub(crate) mod table;
pub(crate) mod tags;
pub(crate) mod termsize;
pub(crate) mod to;
pub(crate) mod to_csv;
pub(crate) mod to_html;
pub(crate) mod to_json;
pub(crate) mod to_md;
pub(crate) mod to_toml;
pub(crate) mod to_tsv;
pub(crate) mod to_url;
pub(crate) mod to_xml;
pub(crate) mod to_yaml;
pub(crate) mod uniq;
pub(crate) mod update;
pub(crate) mod url_;
pub(crate) mod version;
pub(crate) mod where_;
pub(crate) mod which_;
pub(crate) mod with_env;
pub(crate) mod wrap;

pub use autoview::Autoview;
pub use cd::Cd;

pub use ansi::Ansi;
pub use ansi::AnsiStrip;
pub use append::Command as Append;
pub use autoenv::Autoenv;
pub use autoenv_trust::AutoenvTrust;
pub use autoenv_untrust::AutoenvUnTrust;
pub use benchmark::Benchmark;
pub use build_string::BuildString;
pub use cal::Cal;
pub use char_::Char;
pub use chart::Chart;
pub use compact::Compact;
pub use config::{
    Config, ConfigClear, ConfigGet, ConfigPath, ConfigRemove, ConfigSet, ConfigSetInto,
};
pub use cp::Cpy;
pub use date::{Date, DateFormat, DateListTimeZone, DateNow, DateToTable, DateToTimeZone};
pub use debug::Debug;
pub use def::Def;
pub use default::Default;
pub use describe::Describe;
pub use do_::Do;
pub use drop::{Drop, DropColumn};
pub use du::Du;
pub use each::Each;
pub use each::EachGroup;
pub use each::EachWindow;
pub use echo::Echo;
pub use empty::Command as Empty;
pub use if_::If;
pub use into::Into;
pub use into::IntoInt;
pub use nu::NuPlugin;
pub use update::Command as Update;
pub mod kill;
pub use kill::Kill;
pub mod clear;
pub use clear::Clear;
pub mod touch;
pub use all::Command as All;
pub use any::Command as Any;
pub use enter::Enter;
pub use every::Every;
pub use exec::Exec;
pub use exit::Exit;
pub use first::First;
pub use flatten::Command as Flatten;
pub use format::{FileSize, Format};
pub use from::From;
pub use from_csv::FromCsv;
pub use from_eml::FromEml;
pub use from_ics::FromIcs;
pub use from_ini::FromIni;
pub use from_json::FromJson;
pub use from_ods::FromOds;
pub use from_ssv::FromSsv;
pub use from_toml::FromToml;
pub use from_tsv::FromTsv;
pub use from_url::FromUrl;
pub use from_vcf::FromVcf;
pub use from_xlsx::FromXlsx;
pub use from_xml::FromXml;
pub use from_yaml::FromYaml;
pub use from_yaml::FromYml;
pub use get::Command as Get;
pub use group_by::Command as GroupBy;
pub use group_by_date::GroupByDate;
pub use hash_::{Hash, HashBase64, HashMd5};
pub use headers::Headers;
pub use help::Help;
pub use histogram::Histogram;
pub use history::History;
pub use insert::Command as Insert;
pub use keep::{Keep, KeepUntil, KeepWhile};
pub use last::Last;
pub use length::Length;
pub use let_::Let;
pub use let_env::LetEnv;
pub use lines::Lines;
pub use ls::Ls;
pub use math::{
    Math, MathAbs, MathAverage, MathCeil, MathEval, MathFloor, MathMaximum, MathMedian,
    MathMinimum, MathMode, MathProduct, MathRound, MathSqrt, MathStddev, MathSummation,
    MathVariance,
};
pub use merge::Merge;
pub use mkdir::Mkdir;
pub use move_::{Move, Mv};
pub use next::Next;
pub use nth::Nth;
pub use open::Open;
pub use parse::Parse;
pub use path::{
    PathBasename, PathCommand, PathDirname, PathExists, PathExpand, PathExtension, PathFilestem,
    PathJoin, PathType,
};
pub use pivot::Pivot;
pub use prepend::Prepend;
pub use prev::Previous;
pub use pwd::Pwd;
#[cfg(feature = "uuid_crate")]
pub use random::RandomUUID;
pub use random::{Random, RandomBool, RandomChars, RandomDecimal, RandomDice, RandomInteger};
pub use range::Range;
pub use reduce::Reduce;
pub use reject::Reject;
pub use rename::Rename;
pub use reverse::Reverse;
pub use rm::Remove;
pub use roll::{Roll, RollColumn, RollUp};
pub use rotate::{Rotate, RotateCounterClockwise};
pub use run_external::RunExternalCommand;
pub use save::Save;
pub use select::Command as Select;
pub use seq::Seq;
pub use seq_dates::SeqDates;
pub use shells::Shells;
pub use shuffle::Shuffle;
pub use size::Size;
pub use skip::{Skip, SkipUntil, SkipWhile};
pub use sleep::Sleep;
pub use sort_by::SortBy;
pub use source::Source;
pub use split::{Split, SplitChars, SplitColumn, SplitRow};
pub use split_by::SplitBy;
pub use str_::{
    Str, StrCamelCase, StrCapitalize, StrCollect, StrContains, StrDowncase, StrEndsWith,
    StrFindReplace, StrFrom, StrIndexOf, StrKebabCase, StrLPad, StrLength, StrPascalCase, StrRPad,
    StrReverse, StrScreamingSnakeCase, StrSnakeCase, StrStartsWith, StrSubstring, StrToDatetime,
    StrToDecimal, StrToInteger, StrTrim, StrTrimLeft, StrTrimRight, StrUpcase,
};
pub use table::Table;
pub use tags::Tags;
pub use termsize::TermSize;
pub use to::To;
pub use to_csv::ToCsv;
pub use to_html::ToHtml;
pub use to_json::ToJson;
pub use to_md::Command as ToMarkdown;
pub use to_toml::ToToml;
pub use to_tsv::ToTsv;
pub use to_url::ToUrl;
pub use to_xml::ToXml;
pub use to_yaml::ToYaml;
pub use touch::Touch;
pub use uniq::Uniq;
pub use url_::{UrlCommand, UrlHost, UrlPath, UrlQuery, UrlScheme};
pub use version::Version;
pub use where_::Command as Where;
pub use which_::Which;
pub use with_env::WithEnv;
pub use wrap::Wrap;

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
            whole_stream_command(Move),
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
            whole_stream_command(StrFrom),
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
