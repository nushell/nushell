//! Subcommands for working with semantic versions (SemVer 2.0.0).
//!
//! These commands operate on [`Value::SemVer`] values produced by bare semver
//! literals (`1.2.3`) or via [`crate::conversions::into::semver::IntoSemver`].
//!
//! | Command | Description |
//! |---|---|
//! | [`Semver`] | Display this help message |
//! | [`SemverBump`] | Increment major/minor/patch or add/advance prerelease |
//! | [`SemverFromRecord`] | Build a semver from a record with major/minor/patch fields |
//! | [`SemverIsValid`] | Check whether a string is valid semver |
//! | [`SemverMatchReq`] | Check whether a version satisfies a requirement string |
//! | [`SemverSort`] | Sort a list of semver values |
//! | [`SemverToRecord`] | Decompose a semver into its component fields |

use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

mod bump;
mod from_record;
mod is_valid;
mod match_req;
mod semver_;
mod sort;
mod to_record;

pub use bump::SemverBump;
pub use from_record::SemverFromRecord;
pub use is_valid::SemverIsValid;
pub use match_req::SemverMatchReq;
pub use semver_::Semver;
pub use sort::SemverSort;
pub use to_record::SemverToRecord;

/// Extract a copy of the semver version from a [`Value`], with a helpful error when the
/// input is a string (suggesting `into semver` instead of a generic conversion failure).
pub(crate) fn semver_from_input(input: &Value, head: Span) -> Result<semver::Version, ShellError> {
    match input.as_semver() {
        Ok(v) => Ok(v.clone()),
        Err(original_err) => {
            if matches!(input, Value::String { .. }) {
                Err(ShellError::Generic(
                    GenericError::new(
                        "Value is not a semver",
                        "expected a semver value, got a string",
                        head,
                    )
                    .with_help("Use `into semver` to convert a string to a semver value first"),
                ))
            } else {
                Err(original_err)
            }
        }
    }
}
