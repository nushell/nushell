use nu_test_support::{
    fs::Stub::{FileWithContent, FileWithContentToBeTrimmed},
    prelude::*,
};

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn infers_types() -> Result {
    Playground::setup("filter_from_ics_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "calendar.ics",
            "
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
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("open calendar.ics | get events.0 | length")
            .expect_value_eq(2)
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn from_ics_text_to_table() -> Result {
    Playground::setup("filter_from_ics_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "calendar.txt",
            "
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
            ",
        )]);

        let code = r#"
            open calendar.txt
            | from ics
            | get events.0
            | get properties.0
            | where name == "SUMMARY"
            | first
            | get value
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("Maryland Game")
    })
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn from_ics_text_with_linebreak_to_table() -> Result {
    Playground::setup("filter_from_ics_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "calendar.txt",
            "BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20171007T200000Z
DTEND:20171007T233000Z
DTSTAMP:20200319T182138Z
UID:4l80f6dcovnriq38g57g07btid@google.com
CREATED:20170719T202915Z
DESCRIPTION:
LAST-MODIFIED:20170930T190808Z
LOCATION:The Restaurant n
 ear the
  Belltower
SEQUENCE:1
STATUS:CONFIRMED
SUMMARY:Dinner
TRANSP:TRANSPARENT
END:VEVENT
END:VCALENDAR",
        )]);

        let code = r#"
            open calendar.txt
            | from ics
            | get events.0
            | get properties.0
            | where name == "LOCATION"
            | first
            | get value
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("The Restaurant near the Belltower")
    })
}
