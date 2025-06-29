use super::make_local_socket_name;

#[test]
fn local_socket_path_contains_pid() {
    let name = make_local_socket_name("test-string")
        .to_string_lossy()
        .into_owned();
    println!("{name}");
    assert!(name.to_string().contains(&std::process::id().to_string()));
}

#[test]
fn local_socket_path_contains_provided_name() {
    let name = make_local_socket_name("test-string")
        .to_string_lossy()
        .into_owned();
    println!("{name}");
    assert!(name.to_string().contains("test-string"));
}
