//! Unit tests for the crate's internal modules, kept out of the
//! implementation files. Compiled only under `cfg(test)` as part of the crate
//! (declared in `lib.rs`), so they can exercise `pub(crate)` internals directly
//! — unlike the integration tests in the top-level `tests/` directory, which
//! see only the public API.

mod debugger;
mod paths;
mod source_map;
mod variables;
