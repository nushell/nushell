use engine_q::{lex, lite_parse, LexMode, ParserWorkingSet};

fn main() -> std::io::Result<()> {
    if let Some(path) = std::env::args().nth(1) {
        let file = std::fs::read(&path)?;

        // let (output, err) = lex(&file, 0, 0, LexMode::Normal);

        // println!("{:?} tokens, error: {:?}", output, err);

        // let (output, err) = lite_parse(&output);

        // println!("{:?}, error: {:?}", output, err);

        let mut working_set = ParserWorkingSet::new(None);

        let (output, err) = working_set.parse_file(&path, &file);
        println!("{:?} {:?}", output, err);

        Ok(())
    } else {
        println!("specify file to lex");
        Ok(())
    }
}
