use engine_q::{ParserWorkingSet, Signature, SyntaxShape};

fn main() -> std::io::Result<()> {
    if let Some(path) = std::env::args().nth(1) {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);

        let sig =
            Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition");
        working_set.add_decl((b"where").to_vec(), sig);

        let sig = Signature::build("if")
            .required("cond", SyntaxShape::RowCondition, "condition")
            .required("then_block", SyntaxShape::Block, "then block")
            .required(
                "else",
                SyntaxShape::Literal(b"else".to_vec()),
                "else keyword",
            )
            .required("else_block", SyntaxShape::Block, "else block");
        working_set.add_decl((b"if").to_vec(), sig);

        //let file = std::fs::read(&path)?;
        //let (output, err) = working_set.parse_file(&path, file);
        let (output, err) = working_set.parse_source(path.as_bytes());
        println!("{:#?}", output);
        println!("error: {:?}", err);
        // println!("{}", size_of::<Statement>());

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
