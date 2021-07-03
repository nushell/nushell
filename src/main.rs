use engine_q::{ParserWorkingSet, Signature, SyntaxShape};

fn main() -> std::io::Result<()> {
    if let Some(path) = std::env::args().nth(1) {
        let mut working_set = ParserWorkingSet::new(None);

        let sig = Signature::build("foo").named("--jazz", SyntaxShape::Int, "jazz!!", Some('j'));
        working_set.add_decl((b"foo").to_vec(), sig);

        let file = std::fs::read(&path)?;
        let (output, err) = working_set.parse_file(&path, &file);
        //let (output, err) = working_set.parse_source(path.as_bytes());
        println!("{}", output.len());
        println!("error: {:?}", err);

        Ok(())
    } else {
        println!("specify file to lex");
        Ok(())
    }
}
