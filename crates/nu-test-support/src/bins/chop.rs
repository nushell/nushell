use std::io::{self, BufRead};

fn main() {
    if did_chop_arguments() {
        // we are done and don't care about standard input.
        std::process::exit(0);
    }

    // if no arguments given, chop from standard input and exit.
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

fn did_chop_arguments() -> bool {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            println!("{}", chop(arg));
        }

        return true;
    }

    false
}
