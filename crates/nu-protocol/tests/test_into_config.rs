use nu_test_support::{nu, pipeline};

#[test]
fn config_add_unsupported_key() {
    let actual = nu!(cwd: ".", pipeline(
		r#"
		$env.config.foo = 2; 
	"#));

    assert!(actual.err.contains("Error while applying config changes"));
}
