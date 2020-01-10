use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn extracts_fields_from_the_given_the_pattern() {
    Playground::setup("parse_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "key_value_separated_arepa_ingredients.txt",
            r#"
                VAR1=Cheese
                VAR2=JonathanParsed
                VAR3=NushellSecretIngredient
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open key_value_separated_arepa_ingredients.txt
                | parse "{Name}={Value}"
                | nth 1
                | get Value
                | echo $it
            "#
        ));

        assert_eq!(actual, "JonathanParsed");
    })
}
