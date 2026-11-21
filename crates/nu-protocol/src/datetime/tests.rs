use super::*;
use pure_rust_locales::Locale;

fn date(text: &str) -> DateTime {
    DateTime::parse_rfc3339(text).expect("test date parses")
}

fn offset(seconds: i32) -> Offset {
    Offset::from_seconds(seconds).expect("test offset")
}

#[test]
fn text_forms() {
    let d = date("2001-02-03T04:05:06.1+05:30");
    assert_eq!(d.to_string(), "2001-02-03 04:05:06.100 +05:30");
    assert_eq!(format!("{d:?}"), "2001-02-03T04:05:06.100+05:30");
    assert_eq!(d.to_rfc3339(), "2001-02-03T04:05:06.100+05:30");
    assert_eq!(d.to_rfc2822(), "Sat, 3 Feb 2001 04:05:06 +0530");
    assert_eq!(
        serde_json::to_string(&d).expect("serialize"),
        "\"2001-02-03T04:05:06.100+05:30\""
    );

    let utc = date("2001-02-03T04:05:06Z");
    assert_eq!(utc.to_rfc3339(), "2001-02-03T04:05:06+00:00");
    assert_eq!(
        utc.to_rfc3339_opts(SecondsFormat::Millis, true),
        "2001-02-03T04:05:06.000Z"
    );
    assert_eq!(
        serde_json::to_string(&utc).expect("serialize"),
        "\"2001-02-03T04:05:06Z\""
    );

    // RFC 3339 and RFC 2822 round an offset with seconds to the minute, half a minute away from
    // zero, and move the wall clock with it, so the text names the same instant.
    let odd = date("2020-03-04T05:06:07Z").with_offset(offset(5 * 3600 + 30 * 60 + 15));
    assert_eq!(odd.to_string(), "2020-03-04 10:36:22 +05:30:15");
    assert_eq!(odd.to_rfc3339(), "2020-03-04T10:36:07+05:30");
    assert_eq!(odd.to_rfc2822(), "Wed, 4 Mar 2020 10:36:07 +0530");
    assert_eq!(date(&odd.to_rfc3339()), odd);
    let west = date("2020-03-04T05:06:07Z").with_offset(offset(-(5 * 3600 + 30 * 60 + 45)));
    assert_eq!(west.to_rfc3339(), "2020-03-03T23:35:07-05:31");
    assert_eq!(
        serde_json::to_string(&west).expect("serialize"),
        "\"2020-03-03T23:35:07-05:31\""
    );

    let bc = date("0000-01-01T00:00:00Z")
        .checked_sub_nanos(365 * 86_400 * 1_000_000_000)
        .expect("in range");
    assert_eq!(bc.to_rfc3339(), "-0001-01-01T00:00:00+00:00");
}

#[test]
fn serde_round_trip() {
    for text in [
        "\"2001-02-03T04:05:06.123456789-08:00\"",
        "\"2001-02-03T04:05:06Z\"",
        "\"-0044-03-15T00:00:00Z\"",
    ] {
        let d: DateTime = serde_json::from_str(text).expect("deserialize");
        assert_eq!(serde_json::to_string(&d).expect("serialize"), text);
    }
    // Text other languages write: two fraction digits, no colon in the offset, `t` and `z`.
    let d: DateTime = serde_json::from_str("\"2001-02-03T04:05:06.12+0530\"").expect("deserialize");
    assert_eq!(d.to_rfc3339(), "2001-02-03T04:05:06.120+05:30");
    let d: DateTime = serde_json::from_str("\"2001-02-03t04:05:06z\"").expect("deserialize");
    assert_eq!(d.to_rfc3339(), "2001-02-03T04:05:06+00:00");
}

/// Every text form nushell writes reads back as the same instant, over the whole range, at
/// offsets with seconds too.
#[test]
fn text_forms_read_back_over_the_whole_range() {
    let (first, last) = (Timestamp::MIN.as_second(), Timestamp::MAX.as_second());
    let step = (last - first) / 4000;
    let instants = (0..=4000).map(|i| (first + i * step, 0)).chain([
        (first, 0),
        (last, 999_999_999),
        (-1, 500_000_000),
        (0, 0),
    ]);
    let offsets = [0, 19_800, -28_800, 19_815, -1_845, 93_599, -93_599];
    for (seconds, nanos) in instants {
        for seconds_east in offsets {
            let d = DateTime::from_unix_seconds(seconds, nanos, offset(seconds_east))
                .expect("in range");
            let serialized = serde_json::to_string(&d).expect("serialize");
            let back: DateTime = serde_json::from_str(&serialized).expect("deserialize");
            assert_eq!(back, d, "{serialized}");
            for text in [d.to_rfc3339(), format!("{d:?}")] {
                assert_eq!(date(&text), d, "{text}");
            }
        }
    }
}

/// What jiff's reader takes that chrono's didn't, and what it doesn't take.
#[test]
fn reads_jiffs_forms() {
    for (text, read) in [
        ("2001-02-03T04:05Z", "2001-02-03T04:05:00+00:00"),
        ("2001-02-03T04:05:06+0530", "2001-02-03T04:05:06+05:30"),
        ("2001-02-03T04:05:06-08", "2001-02-03T04:05:06-08:00"),
        ("2001-02-03T04:05:06,5Z", "2001-02-03T04:05:06.500+00:00"),
        ("2001-02-03 04:05:06Z", "2001-02-03T04:05:06+00:00"),
        (" 2001-02-03T04:05:06Z ", "2001-02-03T04:05:06+00:00"),
        ("-0044-03-15T12:00:00Z", "-0044-03-15T12:00:00+00:00"),
    ] {
        assert_eq!(
            DateTime::parse_rfc3339(text)
                .map(DateTime::to_rfc3339)
                .as_deref(),
            Ok(read),
            "{text}"
        );
    }
    for text in [
        "2001-02-03T04:05:06",
        "2001-02-03T04:05:06.1234567891Z",
        "2001-02-03T04:05:06 UTC",
        "2001-2-3T04:05:06Z",
        "2001-02-29T04:05:06Z",
        "",
    ] {
        assert!(DateTime::parse_rfc3339(text).is_err(), "{text}");
    }
}

#[test]
fn equality_is_by_instant() {
    use std::hash::{BuildHasher, RandomState};
    let a = date("2001-02-03T04:05:06+05:30");
    let b = date("2001-02-02T22:35:06Z");
    assert_eq!(a, b);
    let hasher = RandomState::new();
    assert_eq!(hasher.hash_one(a), hasher.hash_one(b));
    assert!(date("2001-02-03T04:05:06+05:30") < date("2001-02-03T04:05:06Z"));
}

#[test]
fn second_60_reads_as_59() {
    for (text, read) in [
        ("2016-12-31T23:59:60Z", "2016-12-31T23:59:59+00:00"),
        ("2016-12-31T23:59:60.5Z", "2016-12-31T23:59:59.500+00:00"),
        ("2016-12-31T12:00:60+05:30", "2016-12-31T12:00:59+05:30"),
    ] {
        assert_eq!(date(text).to_rfc3339(), read, "{text}");
    }
    assert!(DateTime::parse_rfc3339("2016-12-31T23:59:61Z").is_err());
    assert_eq!(
        DateTime::from_parts(2016, 12, 31, 23, 59, 60, 500_000_000, Offset::UTC),
        Some(date("2016-12-31T23:59:59.5Z"))
    );
    // chrono's leap second, second 59 with a second's worth of extra nanoseconds, as msgpack
    // timestamps written by nushell before hold it.
    assert_eq!(
        DateTime::from_unix_timestamp(1_483_228_799, 1_500_000_000, Offset::UTC),
        Some(date("2016-12-31T23:59:59.5Z"))
    );
    assert_eq!(
        DateTime::from_unix_timestamp(1_483_228_798, 1_500_000_000, Offset::UTC),
        None
    );
}

#[test]
fn dates_cover_jiffs_range() {
    let first = DateTime::new(Timestamp::MIN, Offset::UTC);
    let last = DateTime::new(Timestamp::MAX, Offset::UTC);
    assert_eq!(first.to_rfc3339(), "-9999-01-02T01:59:59+00:00");
    assert_eq!(last.to_rfc3339(), "9999-12-30T22:00:00.999999999+00:00");
    // At the widest offsets, the wall clock stays within years -9999 to 9999.
    let widest = 25 * 3600 + 59 * 60 + 59;
    assert_eq!(last.with_offset(offset(widest)).parts().year, 9999);
    assert_eq!(first.with_offset(offset(-widest)).parts().year, -9999);
    assert_eq!(last.checked_add_nanos(1), None);
    assert_eq!(first.checked_sub_nanos(1), None);
    assert!(DateTime::parse_rfc3339("9999-12-31T00:00:00Z").is_err());
    assert!("+12345-06-07T08:09:10Z".parse::<DateTime>().is_err());
    assert_eq!(
        date("0000-01-01T00:00:00+05:00").to_rfc3339(),
        "0000-01-01T00:00:00+05:00"
    );
}

#[test]
fn format_specifiers() {
    let d = date("2025-04-18T16:04:32.026490+05:30");
    let f = |spec: &str| d.format_localized(spec, Locale::en_US).expect("formats");
    assert_eq!(f("%f"), "026490000");
    assert_eq!(f("%.f"), ".026490");
    assert_eq!(
        f("%Z %z %:z %::z %:::z"),
        "+05:30 +0530 +05:30 +05:30:00 +05"
    );
    assert_eq!(f("%+"), "2025-04-18T16:04:32.026490+05:30");
    assert_eq!(f("%v"), "18-Apr-2025");
    assert_eq!(f("%J_%Q"), "20250418_160432");
    assert_eq!(f("100%% %Y"), "100% 2025");
    assert_eq!(f("%N %q"), "026490000 2");
    assert!(d.format_localized("%K", Locale::en_US).is_err());
    assert!(d.format_localized("%", Locale::en_US).is_err());

    let de = |spec: &str| d.format_localized(spec, Locale::de_DE).expect("formats");
    assert_eq!(de("%A %B %.3f %p"), "Freitag April ,026 ");
    assert_eq!(de("%x %r"), "18.04.2025 16:04:32");
    // `th_TH` carries `%Ey` in its date format.
    let th = d
        .format_localized("%x %Ey", Locale::th_TH)
        .expect("formats");
    assert_eq!(th, "18/04/25 25");
}

/// Locale formats expand before formatting, without `%E`/`%O` modifiers, which no locale
/// table here has an alternative form for (#15266).
mod locale_formats {
    use super::*;
    use crate::datetime::format::expand_locale_formats as expand;

    #[test]
    fn modifiers_inside_locale_formats_are_dropped() {
        assert_eq!(expand("%x %X", Locale::th_TH), "%d/%m/%y %H:%M:%S");
        assert_eq!(expand("%x %X", Locale::lo_LA), "%d/%m/%y %H:%M:%S");
        assert_eq!(expand("%Ey", Locale::en_US), "%y");
        assert_eq!(expand("%Od", Locale::en_US), "%d");
    }

    #[test]
    fn the_locale_field_order_is_kept() {
        assert_eq!(expand("%x", Locale::th_TH), "%d/%m/%y");
        assert_eq!(expand("%x", Locale::en_US), "%m/%d/%Y");
        assert_eq!(expand("%c", Locale::ja_JP), "%Y年%m月%d日 %H時%M分%S秒");
    }

    /// `de_DE`, `fr_FR` and `nl_NL` have no am/pm format; `az_IR` and `fa_IR` also carry `%O`
    /// in the plain time format that replaces it.
    #[test]
    fn am_pm_time_falls_back_to_the_plain_time_format() {
        assert_eq!(expand("%r", Locale::de_DE), "%T");
        assert_eq!(expand("%r", Locale::fr_FR), "%T");
        assert_eq!(expand("%r", Locale::en_US), "%I:%M:%S %p");
        assert_eq!(expand("%r", Locale::az_IR), "%H:%M:%S");
        assert_eq!(expand("%r", Locale::fa_IR), "%H:%M:%S");
    }

    #[test]
    fn literal_percents_and_other_specifiers_are_kept() {
        assert_eq!(expand("%%x", Locale::th_TH), "%%x");
        assert_eq!(expand("100%%", Locale::th_TH), "100%%");
        assert_eq!(expand("%Y-%m-%d", Locale::th_TH), "%Y-%m-%d");
        assert_eq!(expand("", Locale::th_TH), "");
        assert_eq!(expand("%", Locale::th_TH), "%");
    }

    /// Every locale whose `LC_TIME` formats carry `%E` or `%O` renders.
    #[test]
    fn every_locale_with_a_modifier_renders() {
        let d = date("2026-08-27T13:45:00Z");
        for locale in [
            Locale::az_IR,
            Locale::fa_IR,
            Locale::lo_LA,
            Locale::lzh_TW,
            Locale::mnw_MM,
            Locale::my_MM,
            Locale::or_IN,
            Locale::shn_MM,
            Locale::th_TH,
        ] {
            for spec in ["%x", "%X", "%c", "%r"] {
                assert!(
                    d.format_localized(spec, locale).is_ok(),
                    "{locale:?} {spec}"
                );
            }
        }
    }
}

#[test]
fn humanize() {
    let text = |seconds: i64| HumanTime::from_seconds(seconds).to_string();
    assert_eq!(text(0), "now");
    assert_eq!(text(-10), "now");
    assert_eq!(text(11), "in 11 seconds");
    assert_eq!(text(-46), "a minute ago");
    assert_eq!(text(-91), "2 minutes ago");
    assert_eq!(text(3600), "in an hour");
    assert_eq!(text(-86_400), "a day ago");
    assert_eq!(text(-86_400 * 400), "a year ago");
    assert_eq!(text(-86_400 * 365 * 25), "25 years ago");
}

/// `ParsedDate` reading `text` with `format`, as an instant.
fn instant(text: &str, format: &str) -> Result<String, DateParseError> {
    ParsedDate::parse(text, format)
        .and_then(|parsed| parsed.to_datetime())
        .map(DateTime::to_rfc3339)
}

/// `ParsedDate` reading `text` with `format`, as a wall-clock date and time.
fn wall_clock(text: &str, format: &str) -> Result<String, DateParseError> {
    ParsedDate::parse(text, format)
        .and_then(|parsed| parsed.to_wall_clock())
        .map(DateTime::to_rfc3339)
}

#[test]
fn signed_unix_timestamps() {
    assert_eq!(
        instant("-5", "%s").as_deref(),
        Ok("1969-12-31T23:59:55+00:00")
    );
    // A fraction after a negative timestamp counts back from it.
    assert_eq!(
        instant("-1.5", "%s%.f").as_deref(),
        Ok("1969-12-31T23:59:58.500+00:00")
    );
    assert_eq!(
        instant("1.5", "%s%.f").as_deref(),
        Ok("1970-01-01T00:00:01.500+00:00")
    );
    // `%s` wins over a date and time given too.
    assert_eq!(
        instant("2025-04-19 1744972472", "%F %s").as_deref(),
        Ok("2025-04-18T10:34:32+00:00")
    );
}

#[test]
fn zone_names() {
    for name in ["UTC", "GMT", "utc", "gmt"] {
        assert_eq!(
            instant(&format!("2025-04-18 16:04 {name}"), "%F %H:%M %Z").as_deref(),
            Ok("2025-04-18T16:04:00+00:00"),
            "{name}"
        );
    }
    // An offset, which is how nushell writes `%Z`.
    assert_eq!(
        instant("2025-04-18 16:04 +05:30", "%F %H:%M %Z").as_deref(),
        Ok("2025-04-18T16:04:00+05:30")
    );
    for text in [
        "2025-04-18 16:04 PST",
        "2025-04-18 16:04 UTCx",
        "2025-04-18 16:04 日本",
    ] {
        assert!(instant(text, "%F %H:%M %Z").is_err(), "{text}");
    }
    let read_offset =
        |text: &str, format: &str| ParsedDate::parse(text, format).map(|parsed| parsed.offset());
    assert_eq!(
        read_offset("2025-04-18 +0530", "%F %z"),
        Ok(Some(offset(19_800)))
    );
    assert_eq!(read_offset("2025-04-18", "%F"), Ok(None));
    assert!(read_offset("2025-04-18 +9900", "%F %z").is_err());
    // A `Z` that ends the text is `+00:00`, and `%+` reads RFC 3339.
    assert_eq!(
        instant("2025-04-18T16:04:32Z", "%Y-%m-%dT%H:%M:%S%:z").as_deref(),
        Ok("2025-04-18T16:04:32+00:00")
    );
    assert_eq!(
        instant("2025-04-18t16:04:32.5z", "%+").as_deref(),
        Ok("2025-04-18T16:04:32.500+00:00")
    );
    // A literal `Z` in the format is text, so the reading is a wall clock.
    assert_eq!(
        instant("2025-04-18T16:04:32Z", "%Y-%m-%dT%H:%M:%SZ"),
        Err(DateParseError::NotEnough)
    );
}

#[test]
fn wall_clock_falls_back_only_for_missing_fields() {
    assert_eq!(
        wall_clock("2025-04-18", "%F").as_deref(),
        Ok("2025-04-18T00:00:00+00:00")
    );
    assert_eq!(
        wall_clock("2025-04-18 10:30", "%F %H:%M").as_deref(),
        Ok("2025-04-18T10:30:00+00:00")
    );
    // A bad date or a wrong weekday is an error, not a reading that leaves the field out.
    assert!(wall_clock("2021-02-30 10:00", "%F %H:%M").is_err());
    assert!(wall_clock("2021-03-04 Mon 10:00", "%F %a %H:%M").is_err());
    // A time alone is no wall clock.
    assert_eq!(wall_clock("10:30", "%H:%M"), Err(DateParseError::NotEnough));
    // Minutes left out after an hour are 0, and an hour without AM or PM is AM.
    assert_eq!(
        wall_clock("2021-03-04 10", "%F %H").as_deref(),
        Ok("2021-03-04T10:00:00+00:00")
    );
    assert_eq!(
        wall_clock("2021-03-04 10 PM", "%F %I %p").as_deref(),
        Ok("2021-03-04T22:00:00+00:00")
    );
    assert_eq!(
        wall_clock("2025-04-03 4:04", "%F %l:%M").as_deref(),
        Ok("2025-04-03T04:04:00+00:00")
    );
    // nushell's `%J` and `%Q`, and chrono's `%c`, `%x`, `%X`, `%r` and `%v`.
    assert_eq!(
        wall_clock("20250418_160432", "%J_%Q").as_deref(),
        Ok("2025-04-18T16:04:32+00:00")
    );
    assert_eq!(
        wall_clock("%J 2021-03-04", "%%J %F").as_deref(),
        Ok("2021-03-04T00:00:00+00:00")
    );
    assert_eq!(
        wall_clock("Fri Apr 18 16:04:32 2025", "%c").as_deref(),
        Ok("2025-04-18T16:04:32+00:00")
    );
    assert_eq!(
        wall_clock("04/18/25 04:04:32 PM", "%x %r").as_deref(),
        Ok("2025-04-18T16:04:32+00:00")
    );
    assert_eq!(
        wall_clock("18-Apr-2025 16:04:32", "%v %X").as_deref(),
        Ok("2025-04-18T16:04:32+00:00")
    );
}

#[test]
fn second_60_reads_as_59_with_a_format() {
    assert_eq!(
        wall_clock("2016-12-31 23:59:60.5", "%F %T%.f").as_deref(),
        Ok("2016-12-31T23:59:59.500+00:00")
    );
    assert_eq!(
        instant("2016-12-31 23:59:60 +0530", "%F %T %z").as_deref(),
        Ok("2016-12-31T23:59:59+05:30")
    );
    let time = ParsedDate::parse("23:59:60", "%T")
        .and_then(|parsed| parsed.to_time())
        .expect("a time");
    assert_eq!(time, jiff::civil::time(23, 59, 59, 0));
}

#[test]
fn system_times_round_trip() {
    for text in [
        "9999-12-30T22:00:00Z",
        "1969-12-31T23:59:59.25Z",
        "1900-01-01T00:00:00Z",
        "-0001-06-01T00:00:00Z",
    ] {
        let d = date(text);
        let time = d.to_system_time().expect("SystemTime holds the date");
        assert_eq!(DateTime::from_system_time(time), Some(d), "{text}");
    }
    // NTFS file times reach year 30828, past jiff's range.
    let year_10000 = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(253_402_300_800);
    assert_eq!(DateTime::from_system_time(year_10000), None);
}

#[test]
fn wall_clock_at_dst_changes() {
    let new_york = jiff::tz::TimeZone::get("America/New_York").unwrap();
    let read = |text: &str| -> Option<String> {
        let civil: jiff::civil::DateTime = text.parse().unwrap();
        DateTime::from_civil_in(civil, &new_york).map(DateTime::to_rfc3339)
    };
    // Spring forward at 02:00: the skipped hour starts at 02:00:00, not a second later, and
    // moves forward by the hour.
    assert_eq!(
        read("2025-03-09T01:59:59").as_deref(),
        Some("2025-03-09T01:59:59-05:00")
    );
    assert_eq!(
        read("2025-03-09T02:00:00").as_deref(),
        Some("2025-03-09T03:00:00-04:00")
    );
    assert_eq!(
        read("2025-03-09T02:30:00").as_deref(),
        Some("2025-03-09T03:30:00-04:00")
    );
    // Fall back at 02:00: the hour before it repeats and takes the earlier offset, and
    // 02:00:00 itself happens once.
    assert_eq!(
        read("2025-11-02T01:30:00").as_deref(),
        Some("2025-11-02T01:30:00-04:00")
    );
    assert_eq!(
        read("2025-11-02T02:00:00").as_deref(),
        Some("2025-11-02T02:00:00-05:00")
    );
}

/// The offsets `date to-timezone` reads, as it read them before they shared a reader with
/// `parse_utc_offset` and `%z`.
#[test]
fn zone_offsets() {
    use DateParseError::{Invalid, OutOfRange, TooShort};
    for (text, expected) in [
        ("+05", Ok(18_000)),
        ("+0530", Ok(19_800)),
        ("+05:30", Ok(19_800)),
        ("+05 : 30 ", Ok(19_800)),
        ("+05:30:45", Ok(19_845)),
        ("-00:00:30", Ok(-30)),
        ("-05:30", Ok(-19_800)),
        // Only an ASCII sign, and `Invalid` sends the text on to the time zone names.
        ("\u{2212}05:00", Err(Invalid)),
        ("local", Err(Invalid)),
        ("+05:", Err(Invalid)),
        ("+05::30", Err(Invalid)),
        ("+05:30 x", Err(Invalid)),
        ("+05:61", Err(OutOfRange)),
        ("+24:00", Err(OutOfRange)),
        ("+5", Err(TooShort)),
        ("", Err(TooShort)),
    ] {
        let read = parse_zone_offset(text).map(|offset| offset.seconds());
        assert_eq!(read, expected, "{text:?}");
    }
}

/// Comparisons with chrono, which nushell used before, over a grid of inputs.
mod chrono_compat {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    const OFFSETS: [i32; 7] = [0, 19_800, -28_800, 19_815, -1_800, 86_340, -86_340];
    const YEARS: [i32; 11] = [-9999, -9998, -44, -1, 0, 1, 999, 1969, 2001, 9998, 9999];
    const NANOS: [i32; 8] = [
        0,
        1,
        1_000,
        1_000_000,
        100_000_000,
        120_000_000,
        123_456_789,
        999_999_999,
    ];

    fn pairs() -> impl Iterator<Item = (DateTime, chrono::DateTime<FixedOffset>)> {
        OFFSETS.into_iter().flat_map(|off| {
            YEARS.into_iter().flat_map(move |year| {
                NANOS.into_iter().map(move |nanos| {
                    let ours = DateTime::from_parts(year, 3, 4, 17, 6, 7, nanos, offset(off))
                        .expect("in range");
                    let theirs = FixedOffset::east_opt(off)
                        .expect("offset")
                        .with_ymd_and_hms(year, 3, 4, 17, 6, 7)
                        .single()
                        .expect("chrono date")
                        + chrono::Duration::nanoseconds(nanos.into());
                    (ours, theirs)
                })
            })
        })
    }

    #[test]
    fn text_forms_match() {
        use chrono::SecondsFormat as C;
        let formats = [
            (SecondsFormat::Secs, C::Secs),
            (SecondsFormat::Millis, C::Millis),
            (SecondsFormat::Micros, C::Micros),
            (SecondsFormat::Nanos, C::Nanos),
            (SecondsFormat::AutoSi, C::AutoSi),
        ];
        for (ours, theirs) in pairs() {
            assert_eq!(ours.to_string(), theirs.to_string());
            assert_eq!(format!("{ours:?}"), format!("{theirs:?}"));
            // RFC 3339, RFC 2822 and serde text round an offset with seconds to the minute and
            // move the wall clock with it, where chrono kept the clock and moved the instant.
            let rounded = ours.to_minute_offset().offset().seconds();
            let theirs = theirs.with_timezone(&FixedOffset::east_opt(rounded).expect("offset"));
            assert_eq!(ours.to_rfc3339(), theirs.to_rfc3339());
            for (a, b) in formats {
                for z in [false, true] {
                    assert_eq!(ours.to_rfc3339_opts(a, z), theirs.to_rfc3339_opts(b, z));
                }
            }
            if (0..=9999).contains(&chrono::Datelike::year(&theirs)) {
                assert_eq!(ours.to_rfc2822(), theirs.to_rfc2822());
            }
            assert_eq!(
                serde_json::to_string(&ours).expect("ours"),
                serde_json::to_string(&theirs).expect("theirs")
            );
            if let Some(nanos) = theirs.timestamp_nanos_opt() {
                assert_eq!(ours.unix_nanos(), i128::from(nanos));
            }
        }
    }

    /// The specifiers `format date --list` shows, and some chrono rejects.
    const SPECS: &[&str] = &[
        "%Y",
        "%C",
        "%y",
        "%m",
        "%b",
        "%B",
        "%h",
        "%d",
        "%e",
        "%a",
        "%A",
        "%w",
        "%u",
        "%U",
        "%W",
        "%G",
        "%g",
        "%V",
        "%j",
        "%D",
        "%x",
        "%F",
        "%v",
        "%H",
        "%k",
        "%I",
        "%l",
        "%P",
        "%p",
        "%M",
        "%S",
        "%f",
        "%.f",
        "%.3f",
        "%.6f",
        "%.9f",
        "%3f",
        "%6f",
        "%9f",
        "%R",
        "%T",
        "%X",
        "%r",
        "%Z",
        "%z",
        "%:z",
        "%::z",
        "%:::z",
        "%c",
        "%+",
        "%s",
        "%t",
        "%n",
        "%%",
        "%-d",
        "%_m",
        "%0e",
        "%-I:%M",
        "100%% %Y",
        "%%x",
        "%Ex %EX %Od",
        "%K",
        "%",
    ];

    /// What `format date` did before: expand the locale specifiers, then chrono.
    fn chrono_format(
        date: chrono::DateTime<FixedOffset>,
        spec: &str,
        locale: Locale,
    ) -> Option<String> {
        use std::fmt::Write;
        let expanded = super::format::expand_locale_formats(spec, locale);
        let mut out = String::new();
        write!(out, "{}", date.format_localized(&expanded, locale)).ok()?;
        Some(out)
    }

    #[test]
    fn strftime_matches() {
        let locales = [
            Locale::en_US,
            Locale::de_DE,
            Locale::fr_FR,
            Locale::ja_JP,
            Locale::th_TH,
            Locale::ko_KR,
            Locale::az_IR,
            Locale::fa_IR,
            Locale::POSIX,
        ];
        let mut mismatches = Vec::new();
        for (ours, theirs) in pairs().step_by(7) {
            for locale in locales {
                for spec in SPECS {
                    // chrono prints garbage for the century of a year outside 0 to 9999 (`?6`,
                    // `<3`).
                    if *spec == "%C" && !(0..=9999).contains(&chrono::Datelike::year(&theirs)) {
                        continue;
                    }
                    // `%+` is RFC 3339 text, which moves the wall clock with an offset rounded
                    // to the minute.
                    let theirs = if *spec == "%+" {
                        let rounded = ours.to_minute_offset().offset().seconds();
                        theirs.with_timezone(&FixedOffset::east_opt(rounded).expect("offset"))
                    } else {
                        theirs
                    };
                    let expected = chrono_format(theirs, spec, locale);
                    let actual = ours.format_localized(spec, locale).ok();
                    if actual != expected {
                        mismatches.push(format!(
                            "{spec:?} {locale:?} {theirs:?}: ours {actual:?}, chrono {expected:?}"
                        ));
                    }
                }
            }
        }
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_conversions_round_trip() {
        for (ours, theirs) in pairs() {
            assert_eq!(
                DateTime::try_from(theirs).map(|d| d.to_rfc3339()),
                Ok(ours.to_rfc3339())
            );
            let back: chrono::DateTime<FixedOffset> = ours.into();
            assert_eq!(back.to_rfc3339(), theirs.to_rfc3339());
        }
        let leap: chrono::DateTime<FixedOffset> = "2016-12-31T23:59:60.5+05:30".parse().unwrap();
        assert_eq!(
            DateTime::try_from(leap).map(|d| d.to_rfc3339()),
            Ok("2016-12-31T23:59:59.500+05:30".into())
        );
        let far: chrono::DateTime<FixedOffset> = "+12345-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(DateTime::try_from(far), Err(DateParseError::OutOfRange));
    }

    #[test]
    fn arithmetic_matches() {
        let texts = [
            "2016-12-31T23:59:59.5Z",
            "2017-01-01T00:00:00Z",
            "2016-12-31T23:59:58Z",
            "-0044-03-15T12:00:00.3+05:30",
            "9999-01-01T00:00:00Z",
        ];
        let steps = [
            -1_500_000_000_i64,
            -1_000_000_000,
            -300_000_000,
            0,
            300_000_000,
            700_000_000,
            1_000_000_000,
        ];
        for text in texts {
            let ours: DateTime = text.parse().unwrap();
            let theirs = text.parse::<chrono::DateTime<FixedOffset>>().unwrap();
            for step in steps {
                assert_eq!(
                    ours.checked_add_nanos(step).map(|d| d.to_rfc3339()),
                    theirs
                        .checked_add_signed(chrono::Duration::nanoseconds(step))
                        .map(|d| d.to_rfc3339()),
                    "{text} + {step}"
                );
            }
            for days in [-200_000_i64, -1, 0, 1, 200_000, 4_000_000] {
                let expected = theirs
                    .checked_add_signed(chrono::Duration::days(days))
                    .filter(|d| (-9999..=9999).contains(&chrono::Datelike::year(d)))
                    .map(|d| d.to_rfc3339());
                assert_eq!(
                    ours.checked_add_seconds(days * 86_400)
                        .map(|d| d.to_rfc3339()),
                    expected,
                    "{text} + {days} days"
                );
            }
            for other in texts {
                let other_ours: DateTime = other.parse().unwrap();
                let other_theirs = other.parse::<chrono::DateTime<FixedOffset>>().unwrap();
                let difference = theirs.signed_duration_since(other_theirs);
                assert_eq!(
                    ours.nanos_since(other_ours),
                    i128::from(difference.num_seconds()) * 1_000_000_000
                        + i128::from(difference.subsec_nanos()),
                    "{text} - {other}"
                );
            }
        }
    }

    #[test]
    fn utc_offset_matches() {
        for text in [
            "+05:30",
            "-0800",
            "+05 30",
            "+05:30 trailing",
            "Z",
            "+24:00",
            "+05",
            "05:30",
            "\u{2212}01:00",
        ] {
            let ours = parse_utc_offset(text).map(|offset| offset.seconds()).ok();
            let theirs = text
                .parse::<FixedOffset>()
                .map(|offset| offset.local_minus_utc())
                .ok();
            assert_eq!(ours, theirs, "{text:?}");
        }
    }

    #[test]
    fn humanize_matches() {
        use chrono_humanize::HumanTime as Theirs;
        let mut seconds = vec![0_i64, 1, 10, 11, 45, 46, 90, 91];
        seconds.extend((0..400).map(|i| i * i * 997));
        seconds.extend((0..200).map(|i| i * 86_400 * 7 + 3_600));
        for s in seconds {
            for s in [s, -s] {
                assert_eq!(
                    HumanTime::from_seconds(s).to_string(),
                    Theirs::from(chrono::Duration::seconds(s)).to_string(),
                    "{s} seconds"
                );
            }
        }
    }
}
