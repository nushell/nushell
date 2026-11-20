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

    let odd = date("2020-03-04T05:06:07Z").with_offset(offset(5 * 3600 + 30 * 60 + 15));
    assert_eq!(odd.to_string(), "2020-03-04 10:36:22 +05:30:15");
    assert_eq!(odd.to_rfc3339(), "2020-03-04T10:36:22+05:30");

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
    // Text other languages write: two fraction digits, no colon, `UTC`, a space separator.
    let d: DateTime =
        serde_json::from_str("\"2001-02-03 04:05:06.12 +0530\"").expect("deserialize");
    assert_eq!(d.to_rfc3339(), "2001-02-03T04:05:06.120+05:30");
    let d: DateTime = serde_json::from_str("\"2001-02-03T04:05:06 UTC\"").expect("deserialize");
    assert_eq!(d.to_rfc3339(), "2001-02-03T04:05:06+00:00");
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
fn leap_seconds_are_kept() {
    let d = date("2016-12-31T23:59:60.5Z");
    assert_eq!(d.to_rfc3339(), "2016-12-31T23:59:60.500+00:00");
    assert_eq!(
        d.format("%T %S%.f", Locale::POSIX).unwrap(),
        "23:59:60 60.500"
    );
    assert!(d > date("2016-12-31T23:59:59.999Z"));
    assert!(d < date("2017-01-01T00:00:01Z"));
}

#[test]
fn the_whole_of_years_0_to_9999_is_in_range() {
    let last = date("9999-12-31T23:59:59.999999999Z");
    assert_eq!(last.to_rfc3339(), "9999-12-31T23:59:59.999999999+00:00");
    assert_eq!(
        last.format("%a %F", Locale::POSIX).unwrap(),
        "Fri 9999-12-31"
    );
    assert_eq!(
        date("9999-01-01T00:00:00Z")
            .checked_add_nanos(52 * 7 * 86_400 * 1_000_000_000)
            .map(|d| d.to_rfc3339()),
        Some("9999-12-31T00:00:00+00:00".into())
    );
    assert_eq!(
        date("0000-01-01T00:00:00+05:00").to_rfc3339(),
        "0000-01-01T00:00:00+05:00"
    );
}

#[test]
fn years_past_9999_work_as_chrono_did() {
    let far: DateTime = "+12345-06-07T08:09:10Z".parse().unwrap();
    assert_eq!(far.to_rfc3339(), "+12345-06-07T08:09:10+00:00");
    assert_eq!(
        far.format("%Y %a %j", Locale::POSIX).unwrap(),
        "+12345 Thu 158"
    );
    let zone = jiff::tz::TimeZone::get("America/New_York").unwrap();
    assert_eq!(far.to_time_zone(&zone).offset(), offset(-4 * 3600));
    assert!("+262143-01-01T00:00:00Z".parse::<DateTime>().is_err());
    assert!("-262144-12-31T00:00:00Z".parse::<DateTime>().is_err());
}

#[test]
fn format_specifiers() {
    let d = date("2025-04-18T16:04:32.026490+05:30");
    let f = |spec: &str| d.format(spec, Locale::en_US).expect("formats");
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
    assert!(d.format("%K", Locale::en_US).is_err());
    assert!(d.format("%", Locale::en_US).is_err());

    let de = |spec: &str| d.format(spec, Locale::de_DE).expect("formats");
    assert_eq!(de("%A %B %.3f %p"), "Freitag April ,026 ");
    assert_eq!(de("%x %r"), "18.04.2025 16:04:32");
    // `th_TH` carries `%Ey` in its date format.
    let th = d.format("%x %Ey", Locale::th_TH).expect("formats");
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
                assert!(d.format(spec, locale).is_ok(), "{locale:?} {spec}");
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

#[test]
fn signed_unix_timestamps() {
    let read = |text: &str| {
        ParsedDate::parse(text, "%s")
            .and_then(|parsed| parsed.to_datetime())
            .map(DateTime::to_rfc3339)
    };
    assert_eq!(read("-5").as_deref(), Ok("1969-12-31T23:59:55+00:00"));
    assert_eq!(read("+5").as_deref(), Ok("1970-01-01T00:00:05+00:00"));
    // A fraction after a negative timestamp counts back from it.
    let read = |text: &str| {
        ParsedDate::parse(text, "%s%.f")
            .and_then(|parsed| parsed.to_datetime())
            .map(DateTime::to_rfc3339)
    };
    assert_eq!(read("-1.5").as_deref(), Ok("1969-12-31T23:59:58.500+00:00"));
    assert_eq!(read("-0.5").as_deref(), Ok("1969-12-31T23:59:59.500+00:00"));
    assert_eq!(read("1.5").as_deref(), Ok("1970-01-01T00:00:01.500+00:00"));
    // With the date and time given too, they must name the same instant.
    let read = |text: &str| {
        ParsedDate::parse(text, "%F %T%.f %s")
            .and_then(|parsed| parsed.to_datetime())
            .map(DateTime::to_rfc3339)
    };
    assert_eq!(
        read("1969-12-31 23:59:58.5 -1").as_deref(),
        Ok("1969-12-31T23:59:58.500+00:00")
    );
}

#[test]
fn zone_names() {
    let read = |text: &str, format: &str| {
        ParsedDate::parse(text, format)
            .and_then(|parsed| parsed.to_datetime())
            .map(DateTime::to_rfc3339)
    };
    for name in ["UTC", "gmt", "Utc"] {
        assert_eq!(
            read(&format!("2025-04-18 16:04 {name}"), "%F %H:%M %Z").as_deref(),
            Ok("2025-04-18T16:04:00+00:00"),
            "{name}"
        );
    }
    // An offset, which is how nushell writes `%Z`.
    assert_eq!(
        read("2025-04-18 16:04 +05:30", "%F %H:%M %Z").as_deref(),
        Ok("2025-04-18T16:04:00+05:30")
    );
    for text in [
        "2025-04-18 16:04 PST",
        "2025-04-18 16:04 UTCx",
        "2025-04-18 16:04 日本",
    ] {
        assert_eq!(
            read(text, "%F %H:%M %Z"),
            Err(DateParseError::Invalid),
            "{text}"
        );
    }
    // An offset given twice must agree.
    assert_eq!(
        read("2025-04-18 16:04 +0100 UTC", "%F %H:%M %z %Z"),
        Err(DateParseError::Impossible)
    );
    let offset = |text: &str, format: &str| {
        ParsedDate::parse(text, format).and_then(|parsed| parsed.offset())
    };
    assert_eq!(
        offset("2025-04-18 +0530", "%F %z"),
        Ok(Some(jiff::tz::Offset::from_seconds(19_800).unwrap()))
    );
    assert_eq!(offset("2025-04-18", "%F"), Ok(None));
    assert_eq!(
        offset("2025-04-18 +9900", "%F %z"),
        Err(DateParseError::OutOfRange)
    );
}

#[test]
fn wall_clock_falls_back_only_for_missing_fields() {
    let read = |text: &str, format: &str| {
        ParsedDate::parse(text, format)
            .and_then(|parsed| parsed.to_wall_clock())
            .map(DateTime::to_rfc3339)
    };
    assert_eq!(
        read("2025-04-18", "%F").as_deref(),
        Ok("2025-04-18T00:00:00+00:00")
    );
    assert_eq!(
        read("2025-04-18 10:30", "%F %H:%M").as_deref(),
        Ok("2025-04-18T10:30:00+00:00")
    );
    // A bad date or a contradiction is an error, not a reading that leaves the field out.
    assert_eq!(
        read("2021-02-30 10:00", "%F %H:%M"),
        Err(DateParseError::OutOfRange)
    );
    assert_eq!(
        read("2021-03-04 Mon 10:00", "%F %a %H:%M"),
        Err(DateParseError::Impossible)
    );
}

#[test]
fn wall_clock_at_dst_changes() {
    let new_york = jiff::tz::TimeZone::get("America/New_York").unwrap();
    let read = |text: &str, disambiguation: Disambiguation| -> Option<String> {
        let civil: jiff::civil::DateTime = text.parse().unwrap();
        DateTime::from_civil_in(civil, &new_york, disambiguation).map(DateTime::to_rfc3339)
    };
    let (reject, compatible) = (Disambiguation::Reject, Disambiguation::Compatible);
    // Spring forward at 02:00: the skipped hour starts at 02:00:00, not a second later.
    let skipped = ["2025-03-09T02:00:00", "2025-03-09T02:30:00"];
    for text in skipped {
        assert_eq!(read(text, reject), None, "{text}");
    }
    assert_eq!(
        read(skipped[0], compatible).as_deref(),
        Some("2025-03-09T03:00:00-04:00")
    );
    assert_eq!(
        read("2025-03-09T01:59:59", reject).as_deref(),
        Some("2025-03-09T01:59:59-05:00")
    );
    // Fall back at 02:00: the hour before it repeats, and 02:00:00 itself happens once.
    assert_eq!(read("2025-11-02T01:30:00", reject), None);
    assert_eq!(
        read("2025-11-02T01:30:00", compatible).as_deref(),
        Some("2025-11-02T01:30:00-04:00")
    );
    assert_eq!(
        read("2025-11-02T02:00:00", reject).as_deref(),
        Some("2025-11-02T02:00:00-05:00")
    );
}

/// Comparisons with chrono, which nushell used before, over a grid of inputs.
mod chrono_compat {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    const OFFSETS: [i32; 7] = [0, 19_800, -28_800, 19_815, -1_800, 86_340, -86_340];
    const YEARS: [i32; 17] = [
        -262_142, -12_345, -10_000, -9999, -9998, -44, -1, 0, 1, 999, 1969, 2001, 9998, 9999,
        10_000, 12_345, 262_141,
    ];
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
                    let expected = chrono_format(theirs, spec, locale);
                    let actual = ours.format(spec, locale).ok();
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

    #[test]
    fn parse_matches() {
        let corpus = [
            "2001-02-03T04:05:06Z",
            "2001-02-03t04:05:06z",
            "2001-02-03 04:05:06+05:30",
            "2001-02-03T04:05:06.123456789123-08:00",
            "2001-02-03T04:05:06.5\u{2212}01:00",
            "2001-02-03T04:05:06+0530",
            "2001-02-03T04:05:06+24:00",
            "2001-02-03T04:05:06+23:59",
            "2001-02-03T04:05:06+05:60",
            "2001-02-03T04:05Z",
            "2001-02-29T04:05:06Z",
            "2001-13-03T04:05:06Z",
            "2001-02-03T24:05:06Z",
            "2001-02-03T04:05:61Z",
            "2001-02-03T04:05:06Z ",
            "2001-02-03T04:05:06",
            "2001-02-03",
            "0000-01-01T00:00:00Z",
            "9999-12-31T23:59:59.999999999Z",
            "9999-12-31T23:59:59-05:00",
            "2016-12-31T23:59:60.5Z",
            "2016-12-31T12:00:60Z",
            "2016-12-31T23:58:60Z",
            "+262142-12-31T23:59:59.999999999Z",
            "-262143-01-01T00:00:00Z",
            "+262143-01-01T00:00:00Z",
            "-262143-01-01T00:00:00+01:00",
            "2001-2-3T4:5:6Z",
            "2001-02-03T04:05:06 UTC",
            "2001-02-03T04:05:06 utc",
            "2001-02-03 T04:05:06Z",
            " 2001 - 02 - 03T04 : 05 : 06 +05 30",
            "-0044-03-15T00:00:00Z",
            "+2001-02-03T04:05:06Z",
            "12345-01-01T00:00:00Z",
            "2001-02-03T04:05:06.Z",
            "2001-02-03T04:05:06+05",
            "",
        ];
        for text in corpus {
            let strict = DateTime::parse_rfc3339(text).map(|d| d.to_rfc3339()).ok();
            let chrono_strict = chrono::DateTime::parse_from_rfc3339(text)
                .map(|d| d.to_rfc3339())
                .ok();
            assert_eq!(strict, chrono_strict, "strict {text:?}");
            let relaxed = text.parse::<DateTime>().map(|d| d.to_rfc3339()).ok();
            let chrono_relaxed = text
                .parse::<chrono::DateTime<FixedOffset>>()
                .map(|d| d.to_rfc3339())
                .ok();
            assert_eq!(relaxed, chrono_relaxed, "relaxed {text:?}");
        }
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn chrono_conversions_round_trip() {
        for (ours, theirs) in pairs() {
            assert_eq!(DateTime::from(theirs).to_rfc3339(), ours.to_rfc3339());
            let back: chrono::DateTime<FixedOffset> = ours.into();
            assert_eq!(back.to_rfc3339(), theirs.to_rfc3339());
        }
    }

    #[test]
    fn leap_second_arithmetic_matches() {
        let texts = [
            "2016-12-31T23:59:60Z",
            "2016-12-31T23:59:60.3Z",
            "2016-12-31T23:59:59.5Z",
            "2017-01-01T00:00:00Z",
            "2016-12-31T23:59:58Z",
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
            let theirs: chrono::DateTime<FixedOffset> = text.parse().unwrap();
            for step in steps {
                assert_eq!(
                    ours.checked_add_nanos(step).map(|d| d.to_rfc3339()),
                    theirs
                        .checked_add_signed(chrono::Duration::nanoseconds(step))
                        .map(|d| d.to_rfc3339()),
                    "{text} + {step}"
                );
            }
            for days in [-200_000_i64, -1, 0, 1, 200_000, 100_000_000] {
                assert_eq!(
                    ours.checked_add_seconds(days * 86_400)
                        .map(|d| d.to_rfc3339()),
                    theirs
                        .checked_add_signed(chrono::Duration::days(days))
                        .map(|d| d.to_rfc3339()),
                    "{text} + {days} days"
                );
            }
            for other in texts {
                let other_ours: DateTime = other.parse().unwrap();
                let other_theirs: chrono::DateTime<FixedOffset> = other.parse().unwrap();
                assert_eq!(
                    ours.nanos_since(other_ours),
                    i128::from(
                        theirs
                            .signed_duration_since(other_theirs)
                            .num_nanoseconds()
                            .unwrap()
                    ),
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

/// `ParsedDate` against chrono's four `parse_from_str` functions, which `into datetime
/// --format`, `seq date` and `from kdl` used before.
mod strptime_compat {
    use super::*;
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

    /// Every directive chrono's parser knows, and some it rejects.
    const DIRECTIVES: &[&str] = &[
        "%Y", "%C", "%y", "%G", "%g", "%q", "%m", "%b", "%B", "%h", "%d", "%e", "%a", "%A", "%w",
        "%u", "%U", "%W", "%V", "%j", "%D", "%x", "%F", "%v", "%H", "%k", "%I", "%l", "%P", "%p",
        "%M", "%S", "%f", "%.f", "%.3f", "%.6f", "%.9f", "%3f", "%6f", "%9f", "%R", "%T", "%X",
        "%r", "%Z", "%z", "%:z", "%::z", "%:::z", "%#z", "%c", "%+", "%s", "%t", "%n", "%%", "%-d",
        "%_m", "%0e", "%-p", "%-T", "%#a", "%Ex", "%Q", "%J", "%.2f", "%:", "%",
    ];

    /// Formats people write.
    const FORMATS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S",
        "%F %T%.f %z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
        "%Y-%m-%d %H:%M:%S%.f %z",
        "%d/%m/%Y %I:%M %p",
        "%F %l:%M",
        "%F %H:%M %p",
        "%d.%m.%Y %H:%M %P %z",
        "%a, %d %b %Y %H:%M:%S %z",
        "%A, %B %e, %Y",
        "%Y%m%d%H%M%S",
        "%Y-%j",
        "%G-W%V-%u",
        "%Y %U %a",
        "%Y %W %w",
        "%C%y-%m-%d",
        "%q %Y",
        "%s",
        "%s%.f",
        "%s %f",
        "%s %z",
        "%F %s",
        "%H:%M %s",
        "%F %T %s",
        "%s %z %F %T",
        "%s %S",
        "%s %Y",
        "%s %a",
        "%s %I %p",
        "%Z %F %T",
        "%F %T %#z",
        "%F %T %:z",
        "%+",
        "%c",
        "%D %r",
        "%x %X",
        "%v %R",
    ];

    /// Dates whose chrono text the formats read back.
    const DATES: &[&str] = &[
        "2025-04-18T16:04:32.026490+05:30",
        "2016-12-31T23:59:60.5Z",
        "1969-07-20T20:17:40-04:00",
        "0000-01-01T00:00:00Z",
        "-0044-03-15T12:00:00+00:00",
        "9999-12-31T23:59:59.999999999Z",
        "+12345-06-07T08:09:10.1-09:30",
        "2000-02-29T00:00:00-12:00",
        "2024-12-30T23:59:59+14:00",
        "2021-01-03T12:02:03Z",
        "1970-01-01T00:00:00Z",
    ];

    /// Texts no date formats to: the cases that differed from main before this port.
    const CASES: &[(&str, &str)] = &[
        ("2025-04-03 4:04", "%F %l:%M"),
        ("10:30", "%I:%M"),
        ("2016-12-31 23:59:60", "%F %T"),
        ("23:59:60", "%T"),
        ("2016-12-31 23:59:60.5 +0000", "%F %T%.f %z"),
        ("10:30.5", "%H:%M%.f"),
        ("2025-04-18 10:34 1744972472", "%F %H:%M %s"),
        ("1744972472 Sat", "%s %a"),
        ("1744972472 2024", "%s %Y"),
        ("1744972472 10 PM", "%s %I %p"),
        ("1744972472 +0100 2025-04-18 10:34:32", "%s %z %F %T"),
        ("1483228800 23:59:60", "%s %T"),
        ("1483228799 60", "%s %S"),
        ("2016-12-31 23:59:60 1483228800", "%F %T %s"),
        ("2025-04-18 4:04 PM", "%F %H:%M %p"),
        ("1999 12:00", "%Y %H:%M"),
        ("253402300800", "%s"),
        ("-5", "%s"),
        ("+5", "%s"),
        ("99999999999999999999", "%s"),
        ("+12345-01-01", "%Y-%m-%d"),
        ("12345-01-01", "%Y-%m-%d"),
        ("2025-04-18T16:04:32Z", "%+"),
        ("2025-04-18 16:04:32 UTC", "%+"),
        ("2025-04-18 16:04 +05", "%F %H:%M %#z"),
        ("2025-04-18 16:04 Z", "%F %H:%M %#z"),
        ("2025-04-18 16:04 +0530", "%F %H:%M %:z"),
        ("2025-04-18 16:04 +24:00", "%F %H:%M %z"),
        ("2025-04-18 16:04 +99:59", "%F %H:%M %z"),
        ("19 2025-04-18", "%C %F"),
        ("20 2025-04-18", "%C %F"),
        ("2025-W16-5 16:04", "%G-W%V-%u %H:%M"),
        ("2025-108 16:04", "%Y-%j %H:%M"),
        ("Mon 2025-04-18 16:04", "%a %F %H:%M"),
        ("April 3 2025", "%b %e %Y"),
        ("Apr 3 2025", "%B %e %Y"),
        ("16:04:32 123", "%H:%M:%S %f"),
        ("2025-04-18\t16:04", "%F%t%H:%M"),
        ("2025-04-1816:04", "%F %H:%M"),
        ("2025-04-18 16:04 PST", "%F %H:%M %Z"),
        ("2 2025-04-18", "%q %F"),
        ("-262143-01-01", "%F"),
        ("-262144-12-31", "%F"),
        ("262142-12-31", "%F"),
        ("+262142-12-31 23:59 -01:00", "%F %R %z"),
        ("2025", "%Y"),
        ("", "%Y"),
        ("x", ""),
        ("", ""),
        // Text that isn't ASCII, where a byte slice could split a character.
        ("2025年04月18日", "%Y年%m月%d日"),
        ("2025-04-18 ü", "%F ü"),
        ("ü2025", "%Y"),
        ("Mär 3 2025", "%b %e %Y"),
        ("2025\u{2212}04\u{2212}18", "%Y\u{2212}%m\u{2212}%d"),
        ("2025-04-18 16:04 \u{2212}05:00", "%F %H:%M %z"),
        ("2025-04-18 16:04 \u{2212}05", "%F %H:%M %#z"),
        ("\u{ff12}\u{ff10}\u{ff12}\u{ff15}", "%Y"),
        ("2025-04-18 日本 16:04", "%F %Z %H:%M"),
        ("2025-04-18 16:04 pé", "%F %H:%M %p"),
        ("2025-04-18 16:04.é", "%F %H:%M%.f"),
        ("2025-04-18 16:04:05.1é", "%F %T%.3f"),
        ("é", "%a"),
        ("Moné", "%A"),
        ("2025-04-18Té16:04:05Z", "%+"),
        ("2025-04-18 16:04 +0é", "%F %H:%M %z"),
    ];

    fn chrono_reads(text: &str, format: &str) -> [String; 4] {
        fn show<T>(result: Result<T, chrono::ParseError>, text: impl Fn(T) -> String) -> String {
            result.map_or_else(|err| format!("error: {err}"), text)
        }
        let day = NaiveDate::from_ymd_opt(2000, 1, 1).expect("date");
        [
            show(
                chrono::DateTime::<FixedOffset>::parse_from_str(text, format),
                |d| d.to_rfc3339(),
            ),
            show(NaiveDateTime::parse_from_str(text, format), |d| {
                d.and_utc().to_rfc3339()
            }),
            show(NaiveDate::parse_from_str(text, format), |d| {
                d.and_time(NaiveTime::MIN).and_utc().to_rfc3339()
            }),
            show(NaiveTime::parse_from_str(text, format), |t| {
                day.and_time(t).and_utc().to_rfc3339()
            }),
        ]
    }

    fn our_reads(text: &str, format: &str) -> [String; 4] {
        fn show<T>(result: Result<T, DateParseError>, text: impl Fn(T) -> String) -> String {
            result.map_or_else(|err| format!("error: {err}"), text)
        }
        let parsed = match ParsedDate::parse(text, format) {
            Ok(parsed) => parsed,
            Err(err) => return std::array::from_fn(|_| format!("error: {err}")),
        };
        [
            show(parsed.to_datetime(), |d| d.to_rfc3339()),
            show(parsed.to_naive_datetime(), |d| d.to_rfc3339()),
            show(parsed.to_naive_date(), |d| d.to_rfc3339()),
            show(
                parsed
                    .to_naive_time()
                    .and_then(|t| t.on(2000, 1, 1).ok_or(DateParseError::OutOfRange)),
                |d| d.to_rfc3339(),
            ),
        ]
    }

    /// `text` and variations of it: padded, cut short, with trailing text, in other cases, and
    /// with one digit run changed, which makes fields disagree or go out of range.
    fn variations(text: &str) -> Vec<String> {
        let mut out = vec![
            text.to_string(),
            format!(" {text}"),
            format!("{text}x"),
            text.to_uppercase(),
            text.to_lowercase(),
            text.replacen(' ', "  ", 1),
        ];
        if let Some((cut, _)) = text.char_indices().last() {
            out.push(text[..cut].to_string());
        }
        let bytes = text.as_bytes();
        let mut start = 0;
        while start < bytes.len() {
            if !bytes[start].is_ascii_digit() {
                start += 1;
                continue;
            }
            let end = start
                + bytes[start..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
            let last = bytes[end - 1];
            let bumped = if last == b'9' { b'0' } else { last + 1 };
            let mut changed = bytes.to_vec();
            changed[end - 1] = bumped;
            out.push(String::from_utf8(changed).expect("ASCII digit swap"));
            start = end;
        }
        out
    }

    /// Comparisons left out, and readings that differ.
    #[derive(Default)]
    struct Outcome {
        skipped: usize,
        zone_names: usize,
        compared: usize,
        mismatches: Vec<String>,
    }

    impl Outcome {
        fn assert_ok(mut self) {
            // The count is printed so that a wider skip rule in `compare` shows up.
            let total = self.skipped + self.zone_names + self.compared;
            eprintln!(
                "of {total} comparisons, {} skipped for a sign before %s and {} for %Z",
                self.skipped, self.zone_names
            );
            let count = self.mismatches.len();
            self.mismatches.truncate(40);
            assert!(
                count == 0,
                "{count} mismatches:\n{}",
                self.mismatches.join("\n")
            );
        }
    }

    fn compare(text: &str, format: &str, outcome: &mut Outcome) {
        // `%Z` reads only UTC, GMT and offsets, where chrono skipped any word; `zone_names`
        // checks ours.
        if format.contains("%Z") {
            outcome.zone_names += 1;
            return;
        }
        let ours = our_reads(text, format);
        let theirs = chrono_reads(text, format);
        // chrono read `%s` unsigned, so a sign where it starts made each of chrono's readings
        // fail on that character; `signed_unix_timestamps` checks ours.
        let signed_timestamp = format.contains("%s")
            && text.contains(['-', '+'])
            && theirs
                .iter()
                .all(|read| read == "error: input contains invalid characters");
        if signed_timestamp {
            outcome.skipped += 1;
            return;
        }
        outcome.compared += 1;
        if ours != theirs {
            outcome.mismatches.push(format!(
                "{text:?} with {format:?}:\n  ours   {ours:?}\n  chrono {theirs:?}"
            ));
        }
    }

    /// The chrono text of every date in `format`, where chrono can write it.
    fn texts(format: &str) -> Vec<String> {
        use std::fmt::Write;
        DATES
            .iter()
            .filter_map(|date| {
                let date: chrono::DateTime<FixedOffset> = date.parse().expect("test date");
                let mut out = String::new();
                write!(out, "{}", date.format(format)).ok()?;
                Some(out)
            })
            .collect()
    }

    fn check(formats: impl IntoIterator<Item = String>) {
        let mut outcome = Outcome::default();
        for format in formats {
            for text in texts(&format) {
                for text in variations(&text) {
                    compare(&text, &format, &mut outcome);
                }
            }
        }
        outcome.assert_ok();
    }

    #[test]
    fn cases_match() {
        let mut outcome = Outcome::default();
        for (text, format) in CASES {
            for text in variations(text) {
                compare(&text, format, &mut outcome);
            }
        }
        outcome.assert_ok();
    }

    #[test]
    fn common_formats_match() {
        check(FORMATS.iter().map(|f| f.to_string()));
    }

    #[test]
    fn single_directives_match() {
        check(
            DIRECTIVES
                .iter()
                .flat_map(|d| [d.to_string(), format!("%F %T {d}")]),
        );
    }

    #[test]
    fn directive_pairs_match() {
        check(DIRECTIVES.iter().flat_map(|a| {
            DIRECTIVES
                .iter()
                .flat_map(move |b| [format!("{a} {b}"), format!("{a}{b}")])
        }));
    }
}
