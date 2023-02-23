use nu_protocol::engine::{EngineState, StateWorkingSet};

use crate::*;

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
            Break,
            Commandline,
            Const,
            Continue,
            Def,
            DefEnv,
            Describe,
            Do,
            Echo,
            ErrorMake,
            ExportAlias,
            ExportCommand,
            ExportDef,
            ExportDefEnv,
            ExportExtern,
            ExportUse,
            Extern,
            For,
            Help,
            HelpAliases,
            HelpCommands,
            HelpModules,
            HelpOperators,
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
            Module,
            Mut,
            Return,
            Try,
            Use,
            Version,
            While,
        };

        //#[cfg(feature = "plugin")]
        bind_command!(Register);

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error creating default context: {err:?}");
    }

    engine_state
}
