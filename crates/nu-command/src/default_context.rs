use nu_protocol::engine::{EngineState, StateWorkingSet};

use std::path::Path;

use crate::*;

pub fn create_default_context(cwd: impl AsRef<Path>) -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // If there are commands that have the same name as default declarations,
        // they have to be registered before the main declarations. This helps to make
        // them only accessible if the correct input value category is used with the
        // declaration
        #[cfg(feature = "dataframe")]
        add_dataframe_decls(&mut working_set);

        // Core
        bind_command! {
            Alias,
            Debug,
            Def,
            Describe,
            Do,
            Echo,
            ExportCommand,
            ExportDef,
            ExportEnv,
            For,
            Help,
            Hide,
            History,
            If,
            Ignore,
            Let,
            Metadata,
            Module,
            Source,
            Use,
            Version,
        };

        // Filters
        bind_command! {
            All,
            Any,
            Append,
            Collect,
            Columns,
            Compact,
            Drop,
            DropColumn,
            DropNth,
            Each,
            Empty,
            Every,
            First,
            Flatten,
            Get,
            Keep,
            KeepUntil,
            KeepWhile,
            Last,
            Length,
            Lines,
            Nth,
            ParEach,
            Prepend,
            Range,
            Reduce,
            Reject,
            Reverse,
            Select,
            Shuffle,
            Skip,
            SkipUntil,
            SkipWhile,
            Uniq,
            Update,
            Where,
            Wrap,
            Zip,
        };

        // Path
        bind_command! {
            Path,
            PathBasename,
            PathDirname,
            PathExists,
            PathExpand,
            PathJoin,
            PathParse,
            PathRelativeTo,
            PathSplit,
            PathType,
        };

        // System
        bind_command! {
            Benchmark,
            External,
            Ps,
            Sys,
        };

        // Strings
        bind_command! {
            BuildString,
            Char,
            Decode,
            Format,
            Parse,
            Size,
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
            StrFindReplace,
            StrIndexOf,
            StrKebabCase,
            StrLength,
            StrLpad,
            StrPascalCase,
            StrReverse,
            StrRpad,
            StrScreamingSnakeCase,
            StrSnakeCase,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrUpcase
        };

        // FileSystem
        bind_command! {
            Cd,
            Cp,
            Ls,
            Mkdir,
            Mv,
            Open,
            Rm,
            Save,
            Touch,
        };

        // Platform
        bind_command! {
            Ansi,
            AnsiGradient,
            AnsiStrip,
            Clear,
            Kill,
            Sleep,
        };

        // Date
        bind_command! {
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToTable,
            DateToTimezone,
        };

        // Shells
        bind_command! {
            Enter,
            Exit,
            GotoShell,
            NextShell,
            PrevShell,
            Shells,
        };

        // Formats
        bind_command! {
            From,
            FromCsv,
            FromEml,
            FromIcs,
            FromIni,
            FromJson,
            FromOds,
            FromSsv,
            FromToml,
            FromTsv,
            FromUrl,
            FromVcf,
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
            ToHtml,
            ToJson,
            ToMd,
            ToToml,
            ToTsv,
            ToCsv,
            Touch,
            Use,
            Update,
            Where,
            ToUrl,
            ToXml,
            ToYaml,
        };

        // Viewers
        bind_command! {
            Griddle,
            Table,
        };

        // Conversions
        bind_command! {
            Fmt,
            Into,
            IntoBool,
            IntoBinary,
            IntoDatetime,
            IntoDecimal,
            IntoFilesize,
            IntoInt,
            IntoString,
        };

        // Env
        bind_command! {
            Env,
            LetEnv,
            LoadEnv,
            WithEnv,
        };

        // Math
        bind_command! {
            Math,
            MathAbs,
            MathAvg,
            MathCeil,
            MathEval,
            MathFloor,
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
        };

        // Network
        bind_command! {
            Fetch,
            Url,
            UrlHost,
            UrlPath,
            UrlQuery,
            UrlScheme,
        }

        // Random
        bind_command! {
            Random,
            RandomBool,
            RandomChars,
            RandomDecimal,
            RandomDice,
            RandomInteger,
            RandomUuid,
        };

        // Generators
        bind_command! {
            Cal,
            SeqDate,
        };

        // Hash
        bind_command! {
            Hash,
            HashMd5::default(),
            HashSha256::default(),
        };

        #[cfg(feature = "plugin")]
        bind_command!(Register);

        // This is a WIP proof of concept
        // bind_command!(ListGitBranches, Git, GitCheckout, Source);

        working_set.render()
    };

    let _ = engine_state.merge_delta(delta, None, &cwd);

    engine_state
}
