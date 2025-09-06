use crate::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn create_default_context() -> EngineState {
    let mut engine_state = EngineState::new();

    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);

        macro_rules! bind_command {
            ( $( $command:expr ),* $(,)? ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // Core
        bind_command! {
            Alias,
            Attr,
            AttrCategory,
            AttrDeprecated,
            AttrExample,
            AttrSearchTerms,
            Break,
            Collect,
            Const,
            Continue,
            Def,
            Describe,
            Do,
            Echo,
            Error,
            ErrorMake,
            ExportAlias,
            ExportCommand,
            ExportConst,
            ExportDef,
            ExportExtern,
            ExportUse,
            ExportModule,
            Extern,
            For,
            Hide,
            HideEnv,
            If,
            Ignore,
            Overlay,
            OverlayUse,
            OverlayList,
            OverlayNew,
            OverlayHide,
            Let,
            Loop,
            Match,
            Module,
            Mut,
            Return,
            Scope,
            ScopeAliases,
            ScopeCommands,
            ScopeEngineStats,
            ScopeExterns,
            ScopeModules,
            ScopeVariables,
            Try,
            Use,
            Version,
            While,
        };

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    engine_state
}
