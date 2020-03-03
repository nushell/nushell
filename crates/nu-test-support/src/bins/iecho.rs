use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // println! panics if stdout gets closed, whereas writeln gives us an error
    let mut stdout = io::stdout();
    let _ = args
        .iter()
        .skip(1)
        .cycle()
        .try_for_each(|v| writeln!(stdout, "{}", v));
}
