use std::{
    fmt::Arguments,
    io::{Stderr, Stdout},
};

pub fn parse_args<T>(result: Result<T, lexopt::Error>) -> T {
    match result {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

pub fn exit_help() -> ! {
    std::process::exit(2);
}

pub fn fail(exit_code: i32) -> ! {
    std::process::exit(exit_code);
}

pub trait Print {
    fn print(fmt: Arguments<'_>);
}

impl Print for Stdout {
    fn print(fmt: Arguments<'_>) {
        println!("{fmt}")
    }
}

impl Print for Stderr {
    fn print(fmt: Arguments<'_>) {
        eprintln!("{fmt}")
    }
}

pub fn echo_env<P: Print>(keys: impl IntoIterator<Item = impl AsRef<str>>) {
    for key in keys.into_iter() {
        echo_one_env::<P>(key);
    }
}

pub fn echo_one_env<P: Print>(key: impl AsRef<str>) {
    if let Ok(value) = std::env::var(key.as_ref()) {
        P::print(format_args!("{value}"));
    }
}
