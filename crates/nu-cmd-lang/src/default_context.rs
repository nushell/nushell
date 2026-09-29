use crate::*;
use nu_protocol::engine::{EngineState, StateWorkingSet};

pub fn create_default_context() -> EngineState {
    let engine_state = EngineState::new();
    add_default_context(engine_state)
}

pub fn add_default_context(mut engine_state: EngineState) -> EngineState {
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
            AttrComplete,
            AttrCompleteExternal,
            AttrDeprecated,
            AttrExample,
            AttrInteractive,
            AttrSearchTerms,
            Break,
            Collect,
            Const,
            Continue,
            Def,
            Describe,
            Do,
            Echo,
            EnumConstruct,
            EnumFromRecord,
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

        // Internal commands the parser emits for enum types. They must stay
        // registered (the parser resolves them by `DeclId` scan) but hidden
        // from `help` and completion — users write `Type.variant` and
        // `Type.from-record`, not these names.
        working_set.hide_decls(&[b"enum-construct".to_vec(), b"enum-from-record".to_vec()]);

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    engine_state
}
