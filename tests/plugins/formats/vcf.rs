use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;

#[test]
fn infers_types() {
    Playground::setup("filter_from_vcf_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "contacts.vcf",
            r"
                BEGIN:VCARD
                VERSION:3.0
                FN:John Doe
                N:Doe;John;;;
                EMAIL;TYPE=INTERNET:john.doe99@gmail.com
                item1.ORG:'Alpine Ski Resort'
                item1.X-ABLabel:Other
                item2.TITLE:'Ski Instructor'
                item2.X-ABLabel:Other
                BDAY:19001106
                NOTE:Facebook: john.doe.3\nWebsite: \nHometown: Cleveland\, Ohio
                CATEGORIES:myContacts
                END:VCARD
                BEGIN:VCARD
                VERSION:3.0
                FN:Alex Smith
                N:Smith;Alex;;;
                TEL;TYPE=CELL:(890) 123-4567
                CATEGORIES:Band,myContacts
                END:VCARD
            ",
        )]);

        let cwd = dirs.test();
        let actual = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            "open contacts.vcf | length"
        );

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn from_vcf_text_to_table() {
    Playground::setup("filter_from_vcf_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "contacts.txt",
            r"
                BEGIN:VCARD
                VERSION:3.0
                FN:John Doe
                N:Doe;John;;;
                EMAIL;TYPE=INTERNET:john.doe99@gmail.com
                item1.ORG:'Alpine Ski Resort'
                item1.X-ABLabel:Other
                item2.TITLE:'Ski Instructor'
                item2.X-ABLabel:Other
                BDAY:19001106
                NOTE:Facebook: john.doe.3\nWebsite: \nHometown: Cleveland\, Ohio
                CATEGORIES:myContacts
                END:VCARD
            ",
        )]);

        let cwd = dirs.test();
        let actual = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            r#"
                open contacts.txt
                | from vcf
                | get properties.0
                | where name == "EMAIL"
                | first
                | get value
            "#
        );

        assert_eq!(actual.out, "john.doe99@gmail.com");
    })
}

#[test]
fn from_vcf_text_with_linebreak_to_table() {
    Playground::setup("filter_from_vcf_test_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "contacts.txt",
            r"BEGIN:VCARD
VERSION:3.0
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=INTERNET:john.doe99
 @gmail.com
item1.ORG:'Alpine Ski Resort'
item1.X-ABLabel:Other
item2.TITLE:'Ski Instructor'
item2.X-ABLabel:Other
BDAY:19001106
NOTE:Facebook: john.doe.3\nWebsite: \nHometown: Cleveland\, Ohio
CATEGORIES:myContacts
END:VCARD",
        )]);

        let cwd = dirs.test();
        let actual = nu_with_plugins!(
            cwd: cwd,
            plugin: ("nu_plugin_formats"),
            r#"
                open contacts.txt
                | from vcf
                | get properties.0
                | where name == "EMAIL"
                | first
                | get value
            "#
        );

        assert_eq!(actual.out, "john.doe99@gmail.com");
    })
}
