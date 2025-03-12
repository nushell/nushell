use nu_test_support::nu;
use uuid::Uuid;

#[test]
fn generates_valid_uuid4_by_default() {
    let actual = nu!("random uuid");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 4);
    }
}

#[test]
fn generates_valid_uuid1() {
    let actual = nu!("random uuid -v 1 -m 00:11:22:33:44:55");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 1);
    }
}

#[test]
fn generates_valid_uuid3_with_namespace_and_name() {
    let actual = nu!("random uuid -v 3 -n dns -s example.com");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 3);

        let namespace = Uuid::NAMESPACE_DNS;
        let expected = Uuid::new_v3(&namespace, "example.com".as_bytes());
        assert_eq!(uuid, expected);
    }
}

#[test]
fn generates_valid_uuid4() {
    let actual = nu!("random uuid -v 4");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 4);
    }
}

#[test]
fn generates_valid_uuid5_with_namespace_and_name() {
    let actual = nu!("random uuid -v 5 -n dns -s example.com");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 5);

        let namespace = Uuid::NAMESPACE_DNS;
        let expected = Uuid::new_v5(&namespace, "example.com".as_bytes());
        assert_eq!(uuid, expected);
    }
}

#[test]
fn generates_valid_uuid7() {
    let actual = nu!("random uuid -v 7");
    let result = Uuid::parse_str(actual.out.as_str());
    assert!(result.is_ok());

    if let Ok(uuid) = result {
        assert_eq!(uuid.get_version_num(), 7);
    }
}
