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
fn leap_second_reads_as_last_second() {
    let d = date("2016-12-31T23:59:60.5Z");
    assert_eq!(d.to_rfc3339(), "2016-12-31T23:59:59.500+00:00");
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

/// Comparisons with chrono, which nushell used before, over a grid of inputs.
mod chrono_compat {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    const OFFSETS: [i32; 7] = [0, 19_800, -28_800, 19_815, -1_800, 86_340, -86_340];
    const YEARS: [i16; 9] = [-9998, -44, -1, 0, 1, 999, 1969, 2001, 9998];
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
                    let civil = civil::DateTime::new(year, 3, 4, 17, 6, 7, nanos).expect("civil");
                    let ours = DateTime::from_civil(civil, offset(off)).expect("in range");
                    let theirs = FixedOffset::east_opt(off)
                        .expect("offset")
                        .with_ymd_and_hms(year.into(), 3, 4, 17, 6, 7)
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
                    // chrono prints garbage for the century of a negative year (`?6`).
                    if *spec == "%C" && chrono::Datelike::year(&theirs) < 0 {
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
            "9999-12-30T21:59:59Z",
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
                .ok()
                // jiff stops at year 9999.
                .filter(|t| !t.starts_with('+'));
            assert_eq!(relaxed, chrono_relaxed, "relaxed {text:?}");
        }
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
