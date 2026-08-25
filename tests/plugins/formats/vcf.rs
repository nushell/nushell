use indoc::indoc;
use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn infers_types(playground: Playground) -> Result {
    playground.file(
        "contacts.vcf",
        indoc::indoc! {r"
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
    "},
    )?;
    test()
        .cwd(playground.path())
        .run("open contacts.vcf | length")
        .expect_value_eq(2)
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn from_vcf_text_to_table(playground: Playground) -> Result {
    playground.file(
        "contacts.txt",
        indoc::indoc! {r"
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
    "},
    )?;

    let code = r#"
        open contacts.txt
        | from vcf
        | get properties.0
        | where name == "EMAIL"
        | first
        | get value
    "#;
    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("john.doe99@gmail.com")
}

#[test]
#[deps(NU_PLUGIN_FORMATS)]
fn from_vcf_text_with_linebreak_to_table(playground: Playground) -> Result {
    playground.file(
        "contacts.txt",
        indoc! {r"
            BEGIN:VCARD
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
            END:VCARD
        "},
    )?;

    let code = r#"
        open contacts.txt
        | from vcf
        | get properties.0
        | where name == "EMAIL"
        | first
        | get value
    "#;
    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("john.doe99@gmail.com")
}
