use std::io::{self, BufRead};

fn main() {
    let stdin = io::stdin();

    let mut input = stdin.lock().lines();

    if let Some(Ok(given)) = input.next() {
        if !given.is_empty() {
            println!("{}", chop(&given));
            std::process::exit(0);
        }
    }

    std::process::exit(0);
}

fn chop(word: &str) -> &str {
    let to = word.len() - 1;

    &word[..to]
}
