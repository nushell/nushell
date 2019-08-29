mod helpers;

#[test]
fn external_command() {
    let actual = nu!(
    	cwd: "tests/fixtures",
    	"echo 1"
    );

    assert!(actual.contains("1"));
}
