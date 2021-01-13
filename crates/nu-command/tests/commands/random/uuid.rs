use nu_test_support::{nu, pipeline};
use uuid_crate::Uuid;

#[test]
fn generates_valid_uuid4() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random uuid
        "#
    ));

    let result = Uuid::parse_str(actual.out.as_str());

    assert!(result.is_ok());
}
