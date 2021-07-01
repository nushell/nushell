use engine_q::{lex, lite_parse, LexMode, ParserWorkingSet};

fn main() -> std::io::Result<()> {
    if let Some(path) = std::env::args().nth(1) {
        let mut working_set = ParserWorkingSet::new(None);

        //let file = std::fs::read(&path)?;
        //let (output, err) = working_set.parse_file(&path, &file);
        let (output, err) = working_set.parse_source(path.as_bytes());
        println!("{:?} {:?}", output, err);

        Ok(())
    } else {
        println!("specify file to lex");
        Ok(())
    }
}
