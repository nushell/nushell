use nu_protocol::engine::{EngineState, StateWorkingSet};

use crate::*;

pub fn create_default_context() -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // If there are commands that have the same name as default declarations,
        // they have to be registered before the main declarations. This helps to make
        // them only accessible if the correct input value category is used with the
        // declaration
        #[cfg(feature = "dataframe")]
        bind_command!(DataTypes, DescribeDF, OpenDataFrame, ToDataFrame);

        // TODO: sort default context items categorically
        bind_command!(
            Alias,
            All,
            Any,
            Append,
            Benchmark,
            BuildString,
            Cd,
            Clear,
            Collect,
            Cp,
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToTable,
            DateToTimezone,
            Debug,
            Def,
            Describe,
            Do,
            Drop,
            Each,
            Echo,
            Exit,
            ExportCommand,
            ExportDef,
            ExportEnv,
            External,
            First,
            For,
            Format,
            From,
            FromCsv,
            FromJson,
            FromYaml,
            FromYml,
            FromTsv,
            FromToml,
            FromUrl,
            FromEml,
            FromOds,
            FromIcs,
            FromIni,
            FromVcf,
            FromSsv,
            FromXml,
            FromXlsx,
            Get,
            Griddle,
            Help,
            Hide,
            If,
            Into,
            IntoBinary,
            IntoDecimal,
            IntoFilesize,
            IntoInt,
            IntoString,
            Kill,
            Last,
            Length,
            Let,
            LetEnv,
            Lines,
            Ls,
            Math,
            MathAbs,
            MathAvg,
            MathCeil,
            MathFloor,
            MathEval,
            MathMax,
            MathMedian,
            MathMin,
            MathMode,
            MathProduct,
            MathRound,
            MathSqrt,
            MathStddev,
            MathSum,
            MathVariance,
            Mkdir,
            Module,
            Mv,
            ParEach,
            Parse,
            Ps,
            Range,
            Random,
            Reverse,
            Rm,
            Select,
            Shuffle,
            Size,
            Skip,
            SkipUntil,
            SkipWhile,
            Sleep,
            Source,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            Str,
            StrCamelCase,
            StrCapitalize,
            StrCollect,
            StrContains,
            StrDowncase,
            StrEndswith,
            StrIndexOf,
            StrLength,
            StrFindReplace,
            StrKebabCase,
            StrPascalCase,
            StrScreamingSnakeCase,
            StrSnakeCase,
            StrLpad,
            StrRpad,
            StrStartsWith,
            StrReverse,
            StrSubstring,
            StrUpcase,
            StrTrim,
            Sys,
            Table,
            To,
            ToJson,
            ToUrl,
            ToToml,
            Touch,
            Use,
            Update,
            Where,
            WithEnv,
            Wrap,
            Zip
        );

        #[cfg(feature = "plugin")]
        bind_command!(Register);

        // This is a WIP proof of concept
        // bind_command!(ListGitBranches, Git, GitCheckout, Source);

        working_set.render()
    };

    let _ = engine_state.merge_delta(delta);

    engine_state
}
