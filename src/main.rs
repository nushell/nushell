use std::{cell::RefCell, rc::Rc};

use engine_q::{
    eval_block, NuHighlighter, ParserState, ParserWorkingSet, Signature, Stack, State, SyntaxShape,
};

fn main() -> std::io::Result<()> {
    let parser_state = Rc::new(RefCell::new(ParserState::new()));
    let delta = {
        let parser_state = parser_state.borrow();
        let mut working_set = ParserWorkingSet::new(&*parser_state);

        let sig =
            Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
        working_set.add_decl(sig.into());

        let sig = Signature::build("if")
            .required("cond", SyntaxShape::Expression, "condition")
            .required("then_block", SyntaxShape::Block, "then block")
            .optional(
                "else",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "optional else followed by else block",
            );
        working_set.add_decl(sig.into());

        let sig = Signature::build("let")
            .required("var_name", SyntaxShape::VarWithOptType, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            );
        working_set.add_decl(sig.into());

        let sig = Signature::build("alias")
            .required("var_name", SyntaxShape::Variable, "variable name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by value",
            );
        working_set.add_decl(sig.into());

        let sig = Signature::build("sum").required(
            "arg",
            SyntaxShape::List(Box::new(SyntaxShape::Number)),
            "list of numbers",
        );
        working_set.add_decl(sig.into());

        let sig = Signature::build("build-string").rest(SyntaxShape::String, "list of string");
        working_set.add_decl(sig.into());

        let sig = Signature::build("def")
            .required("def_name", SyntaxShape::String, "definition name")
            .required("params", SyntaxShape::Signature, "parameters")
            .required("block", SyntaxShape::Block, "body of the definition");
        working_set.add_decl(sig.into());

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
        working_set.add_decl(sig.into());

        let sig =
            Signature::build("benchmark").required("block", SyntaxShape::Block, "the block to run");
        working_set.add_decl(sig.into());

        // let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        // working_set.add_decl(sig.into());

        // let sig = Signature::build("bar")
        //     .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        //     .switch("--rock", "rock!!", Some('r'));
        // working_set.add_decl(sig.into());
        let sig = Signature::build("exit");
        working_set.add_decl(sig.into());
        let sig = Signature::build("vars");
        working_set.add_decl(sig.into());
        let sig = Signature::build("decls");
        working_set.add_decl(sig.into());
        let sig = Signature::build("blocks");
        working_set.add_decl(sig.into());
        let sig = Signature::build("stack");
        working_set.add_decl(sig.into());

        let sig = Signature::build("add");
        working_set.add_decl(sig.into());
        let sig = Signature::build("add it");
        working_set.add_decl(sig.into());

        let sig = Signature::build("add it together")
            .required("x", SyntaxShape::Int, "x value")
            .required("y", SyntaxShape::Int, "y value");
        working_set.add_decl(sig.into());

        working_set.render()
    };

    {
        ParserState::merge_delta(&mut *parser_state.borrow_mut(), delta);
    }

    if let Some(path) = std::env::args().nth(1) {
        let parser_state = parser_state;

        let file = std::fs::read(&path)?;

        let (block, delta) = {
            let parser_state = parser_state.borrow();
            let mut working_set = ParserWorkingSet::new(&*parser_state);
            let (output, err) = working_set.parse_file(&path, &file, false);
            if let Some(err) = err {
                eprintln!("Parse Error: {:?}", err);
                std::process::exit(1);
            }
            (output, working_set.render())
        };

        ParserState::merge_delta(&mut *parser_state.borrow_mut(), delta);

        let state = State {
            parser_state: &*parser_state.borrow(),
        };

        let stack = Stack::new();

        match eval_block(&state, stack, &block) {
            Ok(value) => {
                println!("{}", value);
            }
            Err(err) => {
                eprintln!("Eval Error: {:?}", err);
                std::process::exit(1);
            }
        }

        Ok(())
    } else {
        use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};

        let mut line_editor = Reedline::new()
            .with_history(Box::new(FileBackedHistory::with_file(
                1000,
                "history.txt".into(),
            )?))?
            .with_highlighter(Box::new(NuHighlighter {
                parser_state: parser_state.clone(),
            }));

        let prompt = DefaultPrompt::new(1);
        let mut current_line = 1;
        let stack = Stack::new();

        loop {
            let input = line_editor.read_line(&prompt)?;
            match input {
                Signal::Success(s) => {
                    if s.trim() == "exit" {
                        break;
                    } else if s.trim() == "vars" {
                        parser_state.borrow().print_vars();
                        continue;
                    } else if s.trim() == "decls" {
                        parser_state.borrow().print_decls();
                        continue;
                    } else if s.trim() == "blocks" {
                        parser_state.borrow().print_blocks();
                        continue;
                    } else if s.trim() == "stack" {
                        stack.print_stack();
                    }
                    // println!("input: '{}'", s);

                    let (block, delta) = {
                        let parser_state = parser_state.borrow();
                        let mut working_set = ParserWorkingSet::new(&*parser_state);
                        let (output, err) = working_set.parse_file(
                            &format!("line_{}", current_line),
                            s.as_bytes(),
                            false,
                        );
                        if let Some(err) = err {
                            eprintln!("Parse Error: {:?}", err);
                            continue;
                        }
                        (output, working_set.render())
                    };

                    ParserState::merge_delta(&mut *parser_state.borrow_mut(), delta);

                    let state = State {
                        parser_state: &*parser_state.borrow(),
                    };

                    match eval_block(&state, stack.clone(), &block) {
                        Ok(value) => {
                            println!("{}", value);
                        }
                        Err(err) => {
                            eprintln!("Eval Error: {:?}", err);
                        }
                    }
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
