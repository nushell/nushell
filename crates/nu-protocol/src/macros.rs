/// Outputs to standard out
///
/// Note: this exists to differentiate between intentional writing to stdout
/// and stray printlns left by accident
#[macro_export]
macro_rules! out {
    ($($tokens:tt)*) => {
        use std::io::Write;
        print!($($tokens)*);
        let _ = std::io::stdout().flush();
    }
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

#[macro_export]
macro_rules! row {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::indexmap::IndexMap::new();
         $( map.insert($key, $val); )*
         ::nu_protocol::UntaggedValue::row(map).into_untagged_value()
    }}
}
