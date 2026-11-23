//! Reading a time zone given as an offset (`+05:30`), `local`, or an IANA name.

use jiff::tz::{TimeZone, TimeZoneDatabase};
use nu_protocol::{DateParseError, DateTime, parse_zone_offset};
use std::sync::LazyLock;

/// The time zone an IANA name names, in any case. The system's zoneinfo is used when it has the
/// name, since it may be newer; otherwise the copy compiled into nushell, so every name in
/// [`TIME_ZONE_NAMES`] resolves on hosts with older or partial zoneinfo.
pub(crate) fn time_zone_named(name: &str) -> Option<TimeZone> {
    jiff::tz::db()
        .get(name)
        .or_else(|_| TimeZoneDatabase::bundled().get(name))
        .ok()
}

/// The IANA time zone names in jiff's bundled database, sorted, for completions and
/// `date list-timezone`. Command signatures build this list at startup, so it comes from the
/// copy compiled into nushell rather than a walk of the system's zoneinfo directory, and every
/// host lists the same names. Lookups still prefer the system's data; see [`time_zone_named`].
pub(crate) static TIME_ZONE_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut names: Vec<&'static str> = TimeZoneDatabase::bundled()
        .available()
        .map(|name| &*name.as_str().to_owned().leak())
        .collect();
    // `posixrules` is a rules file and `Factory` a placeholder; neither was listed before.
    names.retain(|name| !matches!(*name, "posixrules" | "Factory"));
    names.sort_unstable();
    names.dedup();
    names
});

/// `date` shown in the time zone `s` names: an offset such as `+05:30`, `local`, or an IANA
/// name in any case.
///
/// `local` is the system time zone, at the offset in effect at `date`.
pub fn datetime_in_timezone(date: DateTime, s: &str) -> Result<DateTime, DateParseError> {
    match parse_zone_offset(s) {
        Ok(offset) => Ok(date.with_offset(offset)),
        Err(DateParseError::Invalid) => {
            if s.eq_ignore_ascii_case("local") {
                Ok(date.to_time_zone(&TimeZone::system()))
            } else {
                // The name is trimmed for the newline that ends `/etc/timezone`.
                let time_zone = time_zone_named(s.trim()).ok_or(DateParseError::Invalid)?;
                Ok(date.to_time_zone(&time_zone))
            }
        }
        Err(e) => Err(e),
    }
}
