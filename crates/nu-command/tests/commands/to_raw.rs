use nu_test_support::{nu, playground::Playground};

#[test]
fn test_print_utf8_data() {
    let outcome = nu!("[0x[E697A5], 0x[E69CAC], 0x[E8AA9E]] | to raw | print");
    assert!(outcome.status.success());
    assert_eq!("日本語", outcome.out);
}

#[test]
fn test_binary() {
    Playground::setup("test binary", |dirs, _| {
        let outcome = nu!(
            cwd: dirs.test(),
            "0x[aa bb cc dd] | to raw | save --raw test.bin"
        );
        assert!(outcome.status.success());

        let data = std::fs::read(dirs.test().join("test.bin")).expect("failed to read test.bin");
        assert_eq!(b"\xAA\xBB\xCC\xDD"[..], data);
    })
}

#[test]
fn test_binaries() {
    Playground::setup("test binaries", |dirs, _| {
        let outcome = nu!(
            cwd: dirs.test(),
            "[0x[aa bb] 0x[cc dd]] | to raw | save --raw test.bin"
        );
        assert!(outcome.status.success());

        let data = std::fs::read(dirs.test().join("test.bin")).expect("failed to read test.bin");
        assert_eq!(b"\xAA\xBB\xCC\xDD"[..], data);
    })
}

#[test]
fn test_binary_and_str() {
    Playground::setup("test binary_and_str", |dirs, _| {
        let outcome = nu!(
            cwd: dirs.test(),
            "[0x[ff03] foo 0x[ff03] bar] | to raw | save --raw test.bin"
        );
        assert!(outcome.status.success());

        let data = std::fs::read(dirs.test().join("test.bin")).expect("failed to read test.bin");
        assert_eq!(b"\xff\x03foo\xff\x03bar"[..], data);
    })
}

#[test]
fn test_binary_stream() {
    Playground::setup("test binary_stream", |dirs, _| {
        let outcome = nu!(
            cwd: dirs.test(),
            "seq 1 10 | each { into binary } | to raw | save --raw test.bin"
        );
        assert!(outcome.status.success());

        let data = std::fs::read(dirs.test().join("test.bin")).expect("failed to read test.bin");

        let expectation = (1i64..=10)
            .map(|n| n.to_ne_bytes())
            .flatten()
            .collect::<Vec<u8>>();

        assert_eq!(expectation, data);
    })
}
