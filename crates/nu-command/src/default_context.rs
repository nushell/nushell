use std::{cell::RefCell, rc::Rc};

use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Signature, SyntaxShape,
};

use crate::{Alias, Benchmark, BuildString, Def, For, If, Length, Let, LetEnv};

pub fn create_default_context() -> Rc<RefCell<EngineState>> {
    let engine_state = Rc::new(RefCell::new(EngineState::new()));
    let delta = {
        let engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);

        let sig =
            Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
        working_set.add_decl(sig.predeclare());

        working_set.add_decl(Box::new(If));

        working_set.add_decl(Box::new(Let));

        working_set.add_decl(Box::new(LetEnv));

        working_set.add_decl(Box::new(Alias));

        working_set.add_decl(Box::new(BuildString));

        working_set.add_decl(Box::new(Def));

        working_set.add_decl(Box::new(For));

        working_set.add_decl(Box::new(Benchmark));

        working_set.add_decl(Box::new(Length));

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

        working_set.render()
    };

    {
        EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);
    }

    engine_state
}
