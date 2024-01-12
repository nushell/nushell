#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(not(unix))]
mod non_unix;
#[cfg(not(unix))]
pub use non_unix::*;

mod foreground;
pub use foreground::*;
