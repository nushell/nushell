use std::io::{self, BufRead, Write};

pub fn cococo() {
    let args: Vec<String> = args();

    if args.len() > 1 {
        // Write back out all the arguments passed
        // if given at least 1 instead of chickens
        // speaking co co co.
        println!("{}", &args[1..].join(" "));
    } else {
        println!("cococo");
    }
}

pub fn nonu() {
    args().iter().skip(1).for_each(|arg| print!("{}", arg));
}

pub fn iecho() {
    // println! panics if stdout gets closed, whereas writeln gives us an error
    let mut stdout = io::stdout();
    let _ = args()
        .iter()
        .skip(1)
        .cycle()
        .try_for_each(|v| writeln!(stdout, "{}", v));
}

pub fn fail() {
    std::process::exit(1);
}

pub fn chop() {
    if did_chop_arguments() {
        // we are done and don't care about standard input.
        std::process::exit(0);
    }

    // if no arguments given, chop from standard input and exit.
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        if let Ok(given) = line {
            let chopped = if given.is_empty() {
                &given
            } else {
                let to = given.len() - 1;
                &given[..to]
            };

            if let Err(_e) = writeln!(stdout, "{}", chopped) {
                break;
            }
        }
    }

    std::process::exit(0);
}

fn did_chop_arguments() -> bool {
    let args: Vec<String> = args();

    if args.len() > 1 {
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            let chopped = if arg.is_empty() {
                &arg
            } else {
                let to = arg.len() - 1;
                &arg[..to]
            };

            println!("{}", chopped);
        }

        return true;
    }

    false
}

fn args() -> Vec<String> {
    // skip (--testbin bin_name args)
    std::env::args().skip(2).collect()
}
