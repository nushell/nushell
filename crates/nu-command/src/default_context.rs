use std::{cell::RefCell, rc::Rc};

use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Signature,
};

use crate::*;

pub fn create_default_context() -> Rc<RefCell<EngineState>> {
    let engine_state = Rc::new(RefCell::new(EngineState::new()));
    let delta = {
        let engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);

        macro_rules! bind_command {
            ( $command:expr ) => {
                working_set.add_decl(Box::new($command));
            };
            ( $( $command:expr ),* ) => {
                $( working_set.add_decl(Box::new($command)); )*
            };
        }

        // TODO: sort items categorically
        bind_command!(
            Alias,
            Benchmark,
            BuildString,
            Cd,
            Cp,
            Def,
            Do,
            Each,
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
            Length,
            Let,
            LetEnv,
            Lines,
            Ls,
            Mkdir,
            Module,
            Mv,
            Ps,
            Rm,
            Select,
            Split,
            SplitChars,
            SplitColumn,
            SplitRow,
            Sys,
            Table,
            Touch,
            Use,
            Where,
            Wrap
        );

        // This is a WIP proof of concept
        bind_command!(ListGitBranches, Git, GitCheckout, Source);

        let sig = Signature::build("exit");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("vars");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("decls");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("blocks");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("stack");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("contents");
        working_set.add_decl(sig.predeclare());

        working_set.render()
    };

    {
        EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);
    }

    engine_state
}
