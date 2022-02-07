use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn infers_types() {
    Playground::setup("filter_from_ics_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "calendar.ics",
            r#"
                BEGIN:VCALENDAR
                PRODID:-//Google Inc//Google Calendar 70.9054//EN
                VERSION:2.0
                BEGIN:VEVENT
                DTSTART:20171007T200000Z
                DTEND:20171007T233000Z
                DTSTAMP:20200319T182138Z
                UID:4l80f6dcovnriq38g57g07btid@google.com
                CREATED:20170719T202915Z
                DESCRIPTION:
                LAST-MODIFIED:20170930T190808Z
                LOCATION:
                SEQUENCE:1
                STATUS:CONFIRMED
                SUMMARY:Maryland Game
                TRANSP:TRANSPARENT
                END:VEVENT
                BEGIN:VEVENT
                DTSTART:20171002T010000Z
                DTEND:20171002T020000Z
                DTSTAMP:20200319T182138Z
                UID:2v61g7mij4s7ieoubm3sjpun5d@google.com
                CREATED:20171001T180103Z
                DESCRIPTION:
                LAST-MODIFIED:20171001T180103Z
                LOCATION:
                SEQUENCE:0
                STATUS:CONFIRMED
                SUMMARY:Halloween Wars
                TRANSP:OPAQUE
                END:VEVENT
                END:VCALENDAR
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open calendar.ics
                | get events.0
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn from_ics_text_to_table() {
    Playground::setup("filter_from_ics_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "calendar.txt",
            r#"
                BEGIN:VCALENDAR
                BEGIN:VEVENT
                DTSTART:20171007T200000Z
                DTEND:20171007T233000Z
                DTSTAMP:20200319T182138Z
                UID:4l80f6dcovnriq38g57g07btid@google.com
                CREATED:20170719T202915Z
                DESCRIPTION:
                LAST-MODIFIED:20170930T190808Z
                LOCATION:
                SEQUENCE:1
                STATUS:CONFIRMED
                SUMMARY:Maryland Game
                TRANSP:TRANSPARENT
                END:VEVENT
                END:VCALENDAR
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open calendar.txt
                | from ics
                | get events.0
                | get properties.0
                | where name == "SUMMARY"
                | first
                | get value
            "#
        ));

        assert_eq!(actual.out, "Maryland Game");
    })
}
