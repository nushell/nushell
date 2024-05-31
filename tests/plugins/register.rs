use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;

#[test]
fn help() {
    Playground::setup("help", |dirs, _| {
        let actual = nu_with_plugins!(
        cwd: dirs.test(),
            plugin: ("nu_plugin_example"),
            "example one --help"
        );

        assert!(actual.out.contains("test example 1"));
        assert!(actual.out.contains("Extra usage for example one"));
    })
}

#[test]
fn search_terms() {
    Playground::setup("search_terms", |dirs, _| {
        let actual = nu_with_plugins!(
        cwd: dirs.test(),
            plugin: ("nu_plugin_example"),
            r#"help commands | where name == "example one" | echo $"search terms: ($in.search_terms)""#
        );

        assert!(actual.out.contains("search terms: [example]"));
    })
}
