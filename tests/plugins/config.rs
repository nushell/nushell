use nu_protocol::test_record;
use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn none() -> Result {
    let err = test().run("example config").expect_shell_error()?;
    assert_eq!(err.to_string(), "No config sent");
    Ok(())
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn some() -> Result {
    let code = r#"
        $env.config = {
            plugins: {
                example: {
                    path: "some/path",
                    nested: {
                        bool: true,
                        string: "Hello Example!"
                    }
                }
            }
        }
        example config
    "#;

    test().run(code).expect_value_eq(test_record! {
        "path" => "some/path",
        "nested" => test_record! {
            "bool" => true,
            "string" => "Hello Example!"
        }
    })
}
