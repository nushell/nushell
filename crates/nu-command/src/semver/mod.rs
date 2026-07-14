//! Subcommands for working with semantic versions (SemVer 2.0.0).
//!
//! These commands operate on [`SemverValue`] custom values created via
//! [`crate::conversions::into::semver::IntoSemver`].
//!
//! | Command | Description |
//! |---|---|
//! | [`Semver`] | Display this help message |
//! | [`SemverBump`] | Increment major/minor/patch or add/advance prerelease |

pub mod range;
pub mod value;

mod bump;
mod semver_;

pub use bump::SemverBump;
pub use range::SemverRangeValue;
pub use semver_::Semver;
pub use value::SemverValue;
