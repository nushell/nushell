use std::io;
use std::path::PathBuf;

pub fn current_dir() -> io::Result<PathBuf> {
    std::env::current_dir()
}
