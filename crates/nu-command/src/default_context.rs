use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Signature,
};

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

        // TODO: sort default context items categorically
        bind_command!(
            Alias,
            Benchmark,
            BuildString,
            Cd,
            Collect,
            Cp,
            Date,
            DateFormat,
            DateHumanize,
            DateListTimezones,
            DateNow,
            DateToTable,
            DateToTimezone,
            Def,
            Do,
            Each,
            Echo,
            ExportDef,
            External,
            First,
            For,
            Format,
            From,
            FromCsv,
            FromJson,
            FromTsv,
            Get,
            Griddle,
            Help,
            Hide,
            If,
            Into,
            IntoBinary,
            IntoFilesize,
            IntoInt,
            IntoString,
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
            Ps,
            Range,
            Reverse,
            Rm,
            Select,
            Shuffle,
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
            StrKebabCase,
            StrPascalCase,
            StrScreamingSnakeCase,
            StrSnakeCase,
            Sys,
            Table,
            To,
            ToJson,
            Touch,
            Use,
            Where,
            WithEnv,
            Wrap,
            Zip
        );

        #[cfg(feature = "plugin")]
        bind_command!(Register);

        // This is a WIP proof of concept
        bind_command!(ListGitBranches, Git, GitCheckout, Source);

        let sig = Signature::build("exit");
        working_set.add_decl(sig.predeclare());

        working_set.render()
    };

    engine_state.merge_delta(delta);

    engine_state
}
