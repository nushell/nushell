use nu_test_support::prelude::*;
use uuid::Uuid;

#[test]
fn generates_valid_uuid4_by_default() -> Result {
    let outcome: String = test().run("random uuid")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 4);
    Ok(())
}

#[test]
fn generates_valid_uuid1() -> Result {
    let outcome: String = test().run("random uuid -v 1 -m 00:11:22:33:44:55")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 1);
    Ok(())
}

#[test]
fn generates_valid_uuid3_with_namespace_and_name() -> Result {
    let outcome: String = test().run("random uuid -v 3 -n dns -s example.com")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 3);

    let namespace = Uuid::NAMESPACE_DNS;
    let expected = Uuid::new_v3(&namespace, "example.com".as_bytes());
    assert_eq!(uuid, expected);
    Ok(())
}

#[test]
fn generates_valid_uuid4() -> Result {
    let outcome: String = test().run("random uuid -v 4")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 4);
    Ok(())
}

#[test]
fn generates_valid_uuid5_with_namespace_and_name() -> Result {
    let outcome: String = test().run("random uuid -v 5 -n dns -s example.com")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 5);

    let namespace = Uuid::NAMESPACE_DNS;
    let expected = Uuid::new_v5(&namespace, "example.com".as_bytes());
    assert_eq!(uuid, expected);
    Ok(())
}

#[test]
fn generates_valid_uuid7() -> Result {
    let outcome: String = test().run("random uuid -v 7")?;
    let uuid = Uuid::parse_str(outcome.as_str()).unwrap();
    assert_eq!(uuid.get_version_num(), 7);
    Ok(())
}
