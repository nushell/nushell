use nu_test_support::nu;

#[test]
fn interleave_external_commands() {
    let result = nu!("interleave \
        { nu -n -c 'print hello; print world' | lines | each { 'greeter: ' ++ $in } } \
        { nu -n -c 'print nushell; print rocks' | lines | each { 'evangelist: ' ++ $in } } | \
        each { print }; null");
    assert!(result.out.contains("greeter: hello"), "{}", result.out);
    assert!(result.out.contains("greeter: world"), "{}", result.out);
    assert!(result.out.contains("evangelist: nushell"), "{}", result.out);
    assert!(result.out.contains("evangelist: rocks"), "{}", result.out);
}
