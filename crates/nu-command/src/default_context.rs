use crate::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn add_shell_command_context(mut engine_state: EngineState) -> EngineState {
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
            IsEmpty,
            IsNotEmpty,
            Interleave,
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
            Select,
            Shuffle,
            Skip,
            SkipUntil,
            SkipWhile,
            Sort,
            SortBy,
            SplitList,
            Tee,
            Transpose,
            Uniq,
            UniqBy,
            Upsert,
            Update,
            Values,
            Where,
            Window,
            Wrap,
            Zip,
        };

        // Misc
        bind_command! {
            Panic,
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
            Exec,
            NuCheck,
            Sys,
            UName,

        };

        // Help
        bind_command! {
            Help,
            HelpAliases,
            HelpExterns,
            HelpCommands,
            HelpModules,
            HelpOperators,
            HelpEscapes,
        };

        // Debug
        bind_command! {
            Ast,
            Debug,
            DebugInfo,
            DebugProfile,
            Explain,
            Inspect,
            Metadata,
            MetadataSet,
            TimeIt,
            View,
            ViewFiles,
            ViewSource,
            ViewSpan,
        };

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
            DetectColumns,
            Parse,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            SplitWords,
            Str,
            StrCapitalize,
            StrContains,
            StrDistance,
            StrDowncase,
            StrEndswith,
            StrExpand,
            StrJoin,
            StrReplace,
            StrIndexOf,
            StrLength,
            StrReverse,
            StrStats,
            StrStartsWith,
            StrSubstring,
            StrTrim,
            StrUpcase,
            Format,
            FormatDate,
            FormatDuration,
            FormatFilesize,
        };

        // FileSystem
        bind_command! {
            Cd,
            Ls,
            UMkdir,
            Mktemp,
            UMv,
            UCp,
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
            AnsiLink,
            AnsiStrip,
            Clear,
            Du,
            Input,
            InputList,
            InputListen,
            IsTerminal,
            Kill,
            Sleep,
            TermSize,
            Whoami,
        };

        #[cfg(unix)]
        bind_command! { ULimit };

        // Date
        bind_command! {
            Date,
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
            FromXlsx,
            FromXml,
            FromYaml,
            FromYml,
            To,
            ToCsv,
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
        };

        // Conversions
        bind_command! {
            Fill,
            Into,
            IntoBool,
            IntoBinary,
            IntoCellPath,
            IntoDatetime,
            IntoDuration,
            IntoFloat,
            IntoFilesize,
            IntoInt,
            IntoRecord,
            IntoString,
            IntoGlob,
            IntoValue,
        };

        // Env
        bind_command! {
            ExportEnv,
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
            MathLog,
        };

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
            BytesBuild
        }

        // Network
        bind_command! {
            Http,
            HttpDelete,
            HttpGet,
            HttpHead,
            HttpPatch,
            HttpPost,
            HttpPut,
            HttpOptions,
            Url,
            UrlBuildQuery,
            UrlDecode,
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
            RandomDice,
            RandomFloat,
            RandomInt,
            RandomUuid,
        };

        // Generators
        bind_command! {
            Cal,
            Seq,
            SeqDate,
            SeqChar,
            Generate,
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

        // Removed
        bind_command! {
            LetEnv,
            DateFormat,
        };

        // Stor
        #[cfg(feature = "sqlite")]
        bind_command! {
            Stor,
            StorCreate,
            StorDelete,
            StorExport,
            StorImport,
            StorInsert,
            StorOpen,
            StorReset,
            StorUpdate,
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
