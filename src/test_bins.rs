use std::io::{self, BufRead, Write};

/// Echo's value of env keys from args
/// Example: nu --testbin env_echo FOO BAR
/// If it it's not present echo's nothing
pub fn echo_env() {
    let args = args();
    for arg in args {
        if let Ok(v) = std::env::var(arg) {
            println!("{}", v);
        }
    }
}

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

pub fn meow() {
    let args: Vec<String> = args();

    for arg in args.iter().skip(1) {
        let contents = std::fs::read_to_string(arg).expect("Expected a filepath");
        println!("{}", contents);
    }
}

// A binary version of meow
pub fn meowb() {
    let args: Vec<String> = args();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for arg in args.iter().skip(1) {
        let buf = std::fs::read(arg).expect("Expected a filepath");
        handle.write_all(&buf).expect("failed to write to stdout");
    }
}

// Relays anything received on stdin to stdout
pub fn relay() {
    io::copy(&mut io::stdin().lock(), &mut io::stdout().lock())
        .expect("failed to copy stdin to stdout");
}

pub fn nonu() {
    args().iter().skip(1).for_each(|arg| print!("{}", arg));
}

pub fn repeater() {
    let mut stdout = io::stdout();
    let args = args();
    let mut args = args.iter().skip(1);
    let letter = args.next().expect("needs a character to iterate");
    let count = args.next().expect("need the number of times to iterate");

    let count: u64 = count.parse().expect("can't convert count to number");

    for _ in 0..count {
        let _ = write!(stdout, "{}", letter);
    }
    let _ = stdout.flush();
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

    for given in stdin.lock().lines().flatten() {
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

    std::process::exit(0);
}

fn did_chop_arguments() -> bool {
    let args: Vec<String> = args();

    if args.len() > 1 {
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            let chopped = if arg.is_empty() {
                arg
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
