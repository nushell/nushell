use std::sync::Arc;

use engine_q::{ParserState, ParserWorkingSet, Signature, SyntaxShape};

fn main() -> std::io::Result<()> {
    let mut parser_state = Arc::new(ParserState::new());
    let mut working_set = ParserWorkingSet::new(Some(parser_state.clone()));

    let sig = Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
    working_set.add_decl(sig.into());

    let sig = Signature::build("if")
        .required("cond", SyntaxShape::RowCondition, "condition")
        .required("then_block", SyntaxShape::Block, "then block")
        .required(
            "else",
            SyntaxShape::Literal(b"else".to_vec()),
            "else keyword",
        )
        .required("else_block", SyntaxShape::Block, "else block");
    working_set.add_decl(sig.into());

    let sig = Signature::build("let")
        .required("var_name", SyntaxShape::Variable, "variable name")
        .required("=", SyntaxShape::Literal(b"=".to_vec()), "equals sign")
        .required(
            "value",
            SyntaxShape::Expression,
            "the value to set the variable to",
        );
    working_set.add_decl(sig.into());

    let sig = Signature::build("alias")
        .required("var_name", SyntaxShape::Variable, "variable name")
        .required("=", SyntaxShape::Literal(b"=".to_vec()), "equals sign")
        .required(
            "value",
            SyntaxShape::Expression,
            "the value to set the variable to",
        );
    working_set.add_decl(sig.into());

    let sig = Signature::build("sum").required(
        "arg",
        SyntaxShape::List(Box::new(SyntaxShape::Number)),
        "list of numbers",
    );
    working_set.add_decl(sig.into());

    let sig = Signature::build("def")
        .required("def_name", SyntaxShape::String, "definition name")
        .required("params", SyntaxShape::Signature, "parameters")
        .required("block", SyntaxShape::Block, "body of the definition");
    working_set.add_decl(sig.into());

    // let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
    // working_set.add_decl(sig.into());

    // let sig = Signature::build("bar")
    //     .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
    //     .switch("--rock", "rock!!", Some('r'));
    // working_set.add_decl(sig.into());

    let sig = Signature::build("add");
    working_set.add_decl(sig.into());
    let sig = Signature::build("add it");
    working_set.add_decl(sig.into());

    let sig = Signature::build("add it together")
        .required("x", SyntaxShape::Int, "x value")
        .required("y", SyntaxShape::Int, "y value");
    working_set.add_decl(sig.into());
    ParserState::merge_working_set(&mut parser_state, working_set);

    if let Some(path) = std::env::args().nth(1) {
        // let file = std::fs::read(&path)?;
        // let (output, err) = working_set.parse_file(&path, file);

        let mut working_set = ParserWorkingSet::new(Some(parser_state.clone()));
        let (output, err) = working_set.parse_source(path.as_bytes(), false);
        println!("{:#?}", output);
        println!("error: {:?}", err);

        //println!("working set: {:#?}", working_set);

        // println!("{}", size_of::<Statement>());

        // let engine = Engine::new();
        // let result = engine.eval_block(&output);
        // println!("{:?}", result);

        // let mut buffer = String::new();
        // let stdin = std::io::stdin();
        // let mut handle = stdin.lock();

        // handle.read_to_string(&mut buffer)?;

        Ok(())
    } else {
        use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};

        let mut line_editor =
            Reedline::new().with_history(Box::new(FileBackedHistory::new(1000)))?;

        let prompt = DefaultPrompt::new(1);
        let mut current_line = 1;

        loop {
            let input = line_editor.read_line(&prompt)?;
            match input {
                Signal::Success(s) => {
                    if s.trim() == "exit" {
                        break;
                    }
                    println!("input: '{}'", s);
                    let mut working_set = ParserWorkingSet::new(Some(parser_state.clone()));
                    let (output, err) = working_set.parse_file(
                        &format!("line_{}", current_line),
                        s.as_bytes(),
                        false,
                    );

                    ParserState::merge_working_set(&mut parser_state, working_set);
                    println!("{:#?}", parser_state);

                    println!("{:#?}", output);
                    println!("Error: {:?}", err);
                }
                Signal::CtrlC => {
                    println!("Ctrl-c");
                }
                Signal::CtrlD => {
                    break;
                }
                Signal::CtrlL => {
                    line_editor.clear_screen()?;
                }
            }
            current_line += 1;
        }

        Ok(())
    }
}
