use engine_q::{Engine, ParserWorkingSet, Signature, SyntaxShape};

fn main() -> std::io::Result<()> {
    if let Some(path) = std::env::args().nth(1) {
        let mut working_set = ParserWorkingSet::new(None);

        // let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        // working_set.add_decl(sig.into());

        // let sig = Signature::build("bar")
        //     .named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'))
        //     .switch("--rock", "rock!!", Some('r'));
        // working_set.add_decl(sig.into());

        let sig =
            Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
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
            .required(
                "params",
                SyntaxShape::List(Box::new(SyntaxShape::VarWithOptType)),
                "parameters",
            )
            .required("block", SyntaxShape::Block, "body of the definition");
        working_set.add_decl(sig.into());

        //let file = std::fs::read(&path)?;
        //let (output, err) = working_set.parse_file(&path, file);
        let (output, err) = working_set.parse_source(path.as_bytes());
        println!("{:#?}", output);
        println!("error: {:?}", err);

        println!("working set: {:#?}", working_set);

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
        println!("specify file to lex");
        Ok(())
    }
}
