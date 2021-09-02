use std::{cell::RefCell, rc::Rc};

use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Signature, SyntaxShape,
};

pub fn create_default_context() -> Rc<RefCell<EngineState>> {
    let engine_state = Rc::new(RefCell::new(EngineState::new()));
    let delta = {
        let engine_state = engine_state.borrow();
        let mut working_set = StateWorkingSet::new(&*engine_state);

        let sig =
            Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("if")
            .required("cond", SyntaxShape::Expression, "condition")
            .required("then_block", SyntaxShape::Block, "then block")
            .optional(
                "else",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "optional else followed by else block",
            );
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("let")
            .required("var_name", SyntaxShape::VarWithOptType, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            );
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("let-env")
            .required("var_name", SyntaxShape::String, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::String)),
                "equals sign followed by value",
            );
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("alias")
            .required("name", SyntaxShape::String, "name of the alias")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            );
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("build-string").rest(SyntaxShape::String, "list of string");
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("def")
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("block", SyntaxShape::Block, "body of the definition");
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("for")
            .required(
                "var_name",
                SyntaxShape::Variable,
                "name of the looping variable",
            )
            .required(
                "range",
                SyntaxShape::Keyword(b"in".to_vec(), Box::new(SyntaxShape::Int)),
                "range of the loop",
            )
            .required("block", SyntaxShape::Block, "the block to run");
        working_set.add_decl(sig.predeclare());

        let sig =
            Signature::build("benchmark").required("block", SyntaxShape::Block, "the block to run");
        working_set.add_decl(sig.predeclare());

        // let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        // working_set.add_decl(sig.into());

        // let sig = Signature::build("bar")
        //     .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        //     .switch("--rock", "rock!!", Some('r'));
        // working_set.add_decl(sig.into());
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

        let sig = Signature::build("add");
        working_set.add_decl(sig.predeclare());
        let sig = Signature::build("add it");
        working_set.add_decl(sig.predeclare());

        let sig = Signature::build("add it together")
            .required("x", SyntaxShape::Int, "x value")
            .required("y", SyntaxShape::Int, "y value");
        working_set.add_decl(sig.predeclare());

        working_set.render()
    };

    {
        EngineState::merge_delta(&mut *engine_state.borrow_mut(), delta);
    }

    engine_state
}
