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
            For,
            From,
            FromJson,
            Get,
            Griddle,
            Help,
            Hide,
            If,
            Into,
            IntoBinary,
            IntoFilesize,
            IntoInt,
            Last,
            Length,
            Let,
            LetEnv,
            Lines,
            Ls,
            Math,
            MathAbs,
            MathAvg,
            MathMax,
            MathMin,
            MathProduct,
            MathRound,
            MathSqrt,
            MathSum,
            Mkdir,
            Module,
            Mv,
            ParEach,
            Ps,
            Range,
            Rm,
            Select,
            Size,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            Sys,
            Table,
            To,
            ToJson,
            Touch,
            Use,
            Where,
            Wrap,
            Zip
        );

        // This is a WIP proof of concept
        bind_command!(ListGitBranches, Git, GitCheckout, Source);

        let sig = Signature::build("exit");
        working_set.add_decl(sig.predeclare());

        working_set.render()
    };

    {
        EngineState::merge_delta(&mut engine_state, delta);
    }

    engine_state
}
