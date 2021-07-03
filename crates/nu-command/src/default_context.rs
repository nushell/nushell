use crate::prelude::*;
use nu_engine::whole_stream_command;
use std::error::Error;

pub fn create_default_context(interactive: bool) -> Result<EvaluationContext, Box<dyn Error>> {
    let context = EvaluationContext::basic();

    {
        use crate::commands::*;

        context.add_commands(vec![
            // Fundamentals
            whole_stream_command(NuPlugin),
            whole_stream_command(Let),
            whole_stream_command(LetEnv),
            whole_stream_command(UnletEnv),
            whole_stream_command(LoadEnv),
            whole_stream_command(Def),
            whole_stream_command(Source),
            whole_stream_command(Alias),
            whole_stream_command(Ignore),
            // System/file operations
            whole_stream_command(Exec),
            whole_stream_command(Pwd),
            whole_stream_command(Ls),
            whole_stream_command(Du),
            whole_stream_command(Cd),
            whole_stream_command(Remove),
            whole_stream_command(Open),
            whole_stream_command(Pathvar),
            whole_stream_command(PathvarAdd),
            whole_stream_command(PathvarRemove),
            whole_stream_command(PathvarReset),
            whole_stream_command(PathvarAppend),
            whole_stream_command(PathvarSave),
            whole_stream_command(Config),
            whole_stream_command(ConfigGet),
            whole_stream_command(ConfigSet),
            whole_stream_command(ConfigSetInto),
            whole_stream_command(ConfigClear),
            whole_stream_command(ConfigRemove),
            whole_stream_command(ConfigPath),
            whole_stream_command(Help),
            whole_stream_command(History),
            whole_stream_command(Save),
            whole_stream_command(Touch),
            whole_stream_command(Cpy),
            whole_stream_command(Date),
            whole_stream_command(DateListTimeZone),
            whole_stream_command(DateNow),
            whole_stream_command(DateToTable),
            whole_stream_command(DateToTimeZone),
            whole_stream_command(DateFormat),
            whole_stream_command(Cal),
            whole_stream_command(Mkdir),
            whole_stream_command(Mv),
            whole_stream_command(Kill),
            whole_stream_command(Version),
            whole_stream_command(Clear),
            whole_stream_command(Describe),
            whole_stream_command(Which),
            whole_stream_command(Debug),
            whole_stream_command(WithEnv),
            whole_stream_command(Do),
            whole_stream_command(Sleep),
            // Statistics
            whole_stream_command(Size),
            whole_stream_command(Length),
            whole_stream_command(Benchmark),
            // Metadata
            whole_stream_command(Tags),
            whole_stream_command(Lang),
            // Shells
            whole_stream_command(Next),
            whole_stream_command(Previous),
            whole_stream_command(Shells),
            whole_stream_command(Enter),
            whole_stream_command(Exit),
            // Viz
            whole_stream_command(Chart),
            // Viewers
            whole_stream_command(Autoview),
            whole_stream_command(Table),
            // Text manipulation
            whole_stream_command(Hash),
            whole_stream_command(HashBase64),
            whole_stream_command(HashMd5),
            whole_stream_command(Split),
            whole_stream_command(SplitColumn),
            whole_stream_command(SplitRow),
            whole_stream_command(SplitChars),
            whole_stream_command(Lines),
            whole_stream_command(Echo),
            whole_stream_command(Parse),
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
            whole_stream_command(StrStartsWith),
            whole_stream_command(StrEndsWith),
            whole_stream_command(StrCollect),
            whole_stream_command(StrLength),
            whole_stream_command(StrLPad),
            whole_stream_command(StrReverse),
            whole_stream_command(StrRPad),
            whole_stream_command(StrCamelCase),
            whole_stream_command(StrPascalCase),
            whole_stream_command(StrKebabCase),
            whole_stream_command(StrSnakeCase),
            whole_stream_command(StrScreamingSnakeCase),
            whole_stream_command(BuildString),
            whole_stream_command(Ansi),
            whole_stream_command(AnsiStrip),
            whole_stream_command(AnsiGradient),
            whole_stream_command(Char),
            // Column manipulation
            whole_stream_command(DropColumn),
            whole_stream_command(MoveColumn),
            whole_stream_command(Reject),
            whole_stream_command(Select),
            whole_stream_command(Get),
            whole_stream_command(Update),
            whole_stream_command(Insert),
            whole_stream_command(Into),
            whole_stream_command(IntoBinary),
            whole_stream_command(IntoInt),
            whole_stream_command(IntoString),
            whole_stream_command(SplitBy),
            // Row manipulation
            whole_stream_command(All),
            whole_stream_command(Any),
            whole_stream_command(Reverse),
            whole_stream_command(Append),
            whole_stream_command(Prepend),
            whole_stream_command(SortBy),
            whole_stream_command(GroupBy),
            whole_stream_command(GroupByDate),
            whole_stream_command(First),
            whole_stream_command(Last),
            whole_stream_command(Every),
            whole_stream_command(Nth),
            whole_stream_command(Drop),
            whole_stream_command(Format),
            whole_stream_command(FileSize),
            whole_stream_command(Where),
            whole_stream_command(If),
            whole_stream_command(Compact),
            whole_stream_command(Default),
            whole_stream_command(Skip),
            whole_stream_command(SkipUntil),
            whole_stream_command(SkipWhile),
            whole_stream_command(Keep),
            whole_stream_command(KeepUntil),
            whole_stream_command(KeepWhile),
            whole_stream_command(Range),
            whole_stream_command(Rename),
            whole_stream_command(Uniq),
            whole_stream_command(Each),
            whole_stream_command(EachGroup),
            whole_stream_command(EachWindow),
            whole_stream_command(Empty),
            whole_stream_command(ForIn),
            // Table manipulation
            whole_stream_command(Flatten),
            whole_stream_command(Merge),
            whole_stream_command(Shuffle),
            whole_stream_command(Wrap),
            whole_stream_command(Pivot),
            whole_stream_command(Headers),
            whole_stream_command(Reduce),
            whole_stream_command(Roll),
            whole_stream_command(RollColumn),
            whole_stream_command(RollUp),
            whole_stream_command(Rotate),
            whole_stream_command(RotateCounterClockwise),
            whole_stream_command(Collect),
            // Data processing
            whole_stream_command(Histogram),
            whole_stream_command(Autoenv),
            whole_stream_command(AutoenvTrust),
            whole_stream_command(AutoenvUntrust),
            whole_stream_command(Math),
            whole_stream_command(MathAbs),
            whole_stream_command(MathAverage),
            whole_stream_command(MathEval),
            whole_stream_command(MathMedian),
            whole_stream_command(MathMinimum),
            whole_stream_command(MathMode),
            whole_stream_command(MathMaximum),
            whole_stream_command(MathStddev),
            whole_stream_command(MathSummation),
            whole_stream_command(MathVariance),
            whole_stream_command(MathProduct),
            whole_stream_command(MathRound),
            whole_stream_command(MathFloor),
            whole_stream_command(MathCeil),
            whole_stream_command(MathSqrt),
            // File format output
            whole_stream_command(To),
            whole_stream_command(ToCsv),
            whole_stream_command(ToHtml),
            whole_stream_command(ToJson),
            whole_stream_command(ToMarkdown),
            whole_stream_command(ToToml),
            whole_stream_command(ToTsv),
            whole_stream_command(ToUrl),
            whole_stream_command(ToYaml),
            whole_stream_command(ToXml),
            // File format input
            whole_stream_command(From),
            whole_stream_command(FromCsv),
            whole_stream_command(FromEml),
            whole_stream_command(FromTsv),
            whole_stream_command(FromSsv),
            whole_stream_command(FromIni),
            whole_stream_command(FromJson),
            whole_stream_command(FromOds),
            whole_stream_command(FromToml),
            whole_stream_command(FromUrl),
            whole_stream_command(FromXlsx),
            whole_stream_command(FromXml),
            whole_stream_command(FromYaml),
            whole_stream_command(FromYml),
            whole_stream_command(FromIcs),
            whole_stream_command(FromVcf),
            // "Private" commands (not intended to be accessed directly)
            whole_stream_command(RunExternalCommand { interactive }),
            // Random value generation
            whole_stream_command(Random),
            whole_stream_command(RandomBool),
            whole_stream_command(RandomDice),
            #[cfg(feature = "uuid_crate")]
            whole_stream_command(RandomUUID),
            whole_stream_command(RandomInteger),
            whole_stream_command(RandomDecimal),
            whole_stream_command(RandomChars),
            // Path
            whole_stream_command(PathBasename),
            whole_stream_command(PathCommand),
            whole_stream_command(PathDirname),
            whole_stream_command(PathExists),
            whole_stream_command(PathExpand),
            whole_stream_command(PathJoin),
            whole_stream_command(PathParse),
            whole_stream_command(PathRelativeTo),
            whole_stream_command(PathSplit),
            whole_stream_command(PathType),
            // Url
            whole_stream_command(UrlCommand),
            whole_stream_command(UrlScheme),
            whole_stream_command(UrlPath),
            whole_stream_command(UrlHost),
            whole_stream_command(UrlQuery),
            whole_stream_command(Seq),
            whole_stream_command(SeqDates),
            whole_stream_command(TermSize),
        ]);

        //Dataframe commands
        #[cfg(feature = "dataframe")]
        context.add_commands(vec![
            whole_stream_command(DataFrame),
            whole_stream_command(DataFrameOpen),
            whole_stream_command(DataFrameList),
            whole_stream_command(DataFrameGroupBy),
            whole_stream_command(DataFrameAggregate),
            whole_stream_command(DataFrameShow),
            whole_stream_command(DataFrameSample),
            whole_stream_command(DataFrameJoin),
            whole_stream_command(DataFrameDrop),
            whole_stream_command(DataFrameSelect),
            whole_stream_command(DataFrameDTypes),
            whole_stream_command(DataFrameDummies),
            whole_stream_command(DataFrameFirst),
            whole_stream_command(DataFrameLast),
            whole_stream_command(DataFrameSlice),
            whole_stream_command(DataFrameMelt),
            whole_stream_command(DataFramePivot),
            whole_stream_command(DataFrameWhere),
            whole_stream_command(DataFrameToDF),
            whole_stream_command(DataFrameToSeries),
            whole_stream_command(DataFrameToParquet),
            whole_stream_command(DataFrameToCsv),
            whole_stream_command(DataFrameSort),
            whole_stream_command(DataFrameGet),
            whole_stream_command(DataFrameDropDuplicates),
            whole_stream_command(DataFrameDropNulls),
            whole_stream_command(DataFrameColumn),
            whole_stream_command(DataFrameWithColumn),
            whole_stream_command(DataFrameFilter),
            whole_stream_command(DataFrameSeriesRename),
            whole_stream_command(DataFrameValueCounts),
            whole_stream_command(DataFrameIsNull),
            whole_stream_command(DataFrameIsNotNull),
            whole_stream_command(DataFrameAllTrue),
            whole_stream_command(DataFrameAllFalse),
            whole_stream_command(DataFrameArgMax),
            whole_stream_command(DataFrameArgMin),
            whole_stream_command(DataFrameArgTrue),
            whole_stream_command(DataFrameArgUnique),
            whole_stream_command(DataFrameArgSort),
            whole_stream_command(DataFrameUnique),
            whole_stream_command(DataFrameNUnique),
            whole_stream_command(DataFrameNNull),
            whole_stream_command(DataFrameIsUnique),
            whole_stream_command(DataFrameIsDuplicated),
            whole_stream_command(DataFrameIsIn),
            whole_stream_command(DataFrameShift),
            whole_stream_command(DataFrameSet),
            whole_stream_command(DataFrameNot),
            whole_stream_command(DataFrameTake),
        ]);

        #[cfg(feature = "clipboard-cli")]
        {
            context.add_commands(vec![
                whole_stream_command(crate::commands::Clip),
                whole_stream_command(crate::commands::Paste),
            ]);
        }
    }

    Ok(context)
}
