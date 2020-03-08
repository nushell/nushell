/// Outputs to standard out
///
/// Note: this exists to differentiate between intentional writing to stdout
/// and stray printlns left by accident
#[macro_export]
macro_rules! out {
    ($($tokens:tt)*) => { print!($($tokens)*) }
}

/// Outputs to standard out with a newline added
///
/// Note: this exists to differentiate between intentional writing to stdout
/// and stray printlns left by accident
#[macro_export]
macro_rules! outln {
    ($($tokens:tt)*) => { println!($($tokens)*) }
}

/// Outputs to standard error
///
/// Note: this exists to differentiate between intentional writing to stdout
/// and stray printlns left by accident
#[macro_export]
macro_rules! errln {
    ($($tokens:tt)*) => { eprintln!($($tokens)*) }
}
