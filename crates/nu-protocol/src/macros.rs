// These macros exist to differentiate between intentional writing to stdout
// and stray printlns left by accident

#[macro_export]
macro_rules! outln {
    ($($tokens:tt)*) => { println!($($tokens)*) }
}

#[macro_export]
macro_rules! errln {
    ($($tokens:tt)*) => { eprintln!($($tokens)*) }
}
