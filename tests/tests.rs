mod helpers;

use helpers::in_directory as cwd;

#[test]
fn external_num() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.Height | echo $it");
    
    assert_eq!(output, "10");
}

#[test]
fn inc_plugin() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.Height | inc | echo $it");

    assert_eq!(output, "11");
}
