use nu_protocol::engine::{EngineState, StateWorkingSet};

use crate::*;

pub fn create_default_context() -> EngineState {
    let mut engine_state = nu_cmd_lang::create_default_context();

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

        // Database-related
        // Adds all related commands to query databases
        #[cfg(feature = "sqlite")]
        add_database_decls(&mut working_set);

        // Charts
        bind_command! {
            Histogram
        }

        // Filters
        bind_command! {
            All,
            Any,
            Append,
            Columns,
            Compact,
            Default,
            Drop,
            DropColumn,
            DropNth,
            Each,
            EachWhile,
            Empty,
            Enumerate,
            Every,
            Filter,
            Find,
            First,
            Flatten,
            Get,
            Group,
            GroupBy,
            Headers,
            Insert,
            Items,
            Join,
            SplitBy,
            Take,
            Merge,
            Move,
            TakeWhile,
            TakeUntil,
            Last,
            Length,
            Lines,
            ParEach,
            Prepend,
            Range,
            Reduce,
            Reject,
            Rename,
            Reverse,
            Roll,
            RollDown,
            RollUp,
            RollLeft,
            RollRight,
            Rotate,
            Select,
            Shuffle,
            Skip,
            SkipUntil,
            SkipWhile,
            Sort,
            SortBy,
            SplitList,
            Transpose,
            Uniq,
            UniqBy,
            Upsert,
            Update,
            UpdateCells,
            Values,
            Where,
            Window,
            Wrap,
            Zip,
        };

        // Misc
        bind_command! {
            Source,
            Tutor,
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
            Complete,
            External,
            NuCheck,
            Sys,
        };

        // Debug
        bind_command! {
            Ast,
            Debug,
            Explain,
            Inspect,
            Metadata,
            Profile,
            TimeIt,
            View,
            ViewFiles,
            ViewSource,
            ViewSpan,
        };

        #[cfg(unix)]
        bind_command! { Exec }

        #[cfg(windows)]
        bind_command! { RegistryQuery }

        #[cfg(any(
            target_os = "android",
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        ))]
        bind_command! { Ps };

        #[cfg(feature = "which-support")]
        bind_command! { Which };

        // Strings
        bind_command! {
            Char,
            Decode,
            Encode,
            DecodeBase64,
            EncodeBase64,
            DecodeHex,
            EncodeHex,
            DetectColumns,
            Format,
            FileSize,
            Parse,
            Size,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            SplitWords,
            Str,
            StrCamelCase,
            StrCapitalize,
            StrContains,
            StrDistance,
            StrDowncase,
            StrEndswith,
            StrJoin,
            StrReplace,
            StrIndexOf,
            StrKebabCase,
            StrLength,
            StrPascalCase,
            StrReverse,
            StrScreamingSnakeCase,
            StrSnakeCase,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrTitleCase,
            StrUpcase
        };

        // Bits
        bind_command! {
            Bits,
            BitsAnd,
            BitsNot,
            BitsOr,
            BitsXor,
            BitsRotateLeft,
            BitsRotateRight,
            BitsShiftLeft,
            BitsShiftRight,
        }

        // Bytes
        bind_command! {
            Bytes,
            BytesLen,
            BytesStartsWith,
            BytesEndsWith,
            BytesReverse,
            BytesReplace,
            BytesAdd,
            BytesAt,
            BytesIndexOf,
            BytesCollect,
            BytesRemove,
            BytesBuild,
        }

        // FileSystem
        bind_command! {
            Cd,
            Cp,
            Ls,
            Mkdir,
            Mv,
            Open,
            Start,
            Rm,
            Save,
            Touch,
            Glob,
            Watch,
        };

        // Platform
        bind_command! {
            Ansi,
            AnsiGradient,
            AnsiStrip,
            AnsiLink,
            Clear,
            Du,
            Input,
            InputList,
            Kill,
            Sleep,
            TermSize,
        };

        // Date
        bind_command! {
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToRecord,
            DateToTable,
            DateToTimezone,
        };

        // Shells
        bind_command! {
            Exit,
        };

        // Formats
        bind_command! {
            From,
            FromCsv,
            FromJson,
            FromNuon,
            FromOds,
            FromSsv,
            FromToml,
            FromTsv,
            FromUrl,
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
            ToHtml,
            ToJson,
            ToMd,
            ToNuon,
            ToText,
            ToToml,
            ToTsv,
            Touch,
            Upsert,
            Where,
            ToXml,
            ToYaml,
        };

        // Viewers
        bind_command! {
            Griddle,
            Table,
            Explore,
        };

        // Conversions
        bind_command! {
            Fill,
            Fmt,
            Into,
            IntoBool,
            IntoBinary,
            IntoDatetime,
            IntoDecimal,
            IntoDuration,
            IntoFilesize,
            IntoInt,
            IntoRecord,
            IntoString,
        };

        // Env
        bind_command! {
            ExportEnv,
            LetEnv,
            LoadEnv,
            SourceEnv,
            WithEnv,
            ConfigNu,
            ConfigEnv,
            ConfigMeta,
            ConfigReset,
        };

        // Math
        bind_command! {
            Math,
            MathAbs,
            MathAvg,
            MathCeil,
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
            MathSin,
            MathCos,
            MathTan,
            MathSinH,
            MathCosH,
            MathTanH,
            MathArcSin,
            MathArcCos,
            MathArcTan,
            MathArcSinH,
            MathArcCosH,
            MathArcTanH,
            MathPi,
            MathTau,
            MathEuler,
            MathExp,
            MathLn,
            MathLog,
        };

        // Network
        bind_command! {
            Http,
            HttpDelete,
            HttpGet,
            HttpHead,
            HttpPatch,
            HttpPost,
            HttpPut,
            Url,
            UrlBuildQuery,
            UrlEncode,
            UrlJoin,
            UrlParse,
            Port,
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
            Seq,
            SeqDate,
            SeqChar,
        };

        // Hash
        bind_command! {
            Hash,
            HashMd5::default(),
            HashSha256::default(),
        };

        // Experimental
        bind_command! {
            IsAdmin,
        };

        // Deprecated
        bind_command! {
            HashBase64,
            LPadDeprecated,
            MathEvalDeprecated,
            RPadDeprecated,
            StrCollectDeprecated,
            StrDatetimeDeprecated,
            StrDecimalDeprecated,
            StrFindReplaceDeprecated,
            StrIntDeprecated,
        };

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    // Cache the table decl id so we don't have to look it up later
    let table_decl_id = engine_state.find_decl("table".as_bytes(), &[]);
    engine_state.table_decl_id = table_decl_id;

    engine_state
}
