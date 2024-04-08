use super::make_local_socket_path;

#[test]
fn local_socket_path_contains_pid() {
    let path = make_local_socket_path("test-string");
    println!("{}", path.display());
    assert!(path
        .display()
        .to_string()
        .contains(&std::process::id().to_string()));
}

#[test]
fn local_socket_path_contains_provided_name() {
    let path = make_local_socket_path("test-string");
    println!("{}", path.display());
    assert!(path.display().to_string().contains("test-string"));
}

#[cfg(windows)]
#[test]
fn make_local_socket_path_correct_filesystem() {
    let path = make_local_socket_path("test");
    assert!(
        path.starts_with(r"\\.\pipe\"),
        "incorrect path: {}",
        path.display()
    );
}
