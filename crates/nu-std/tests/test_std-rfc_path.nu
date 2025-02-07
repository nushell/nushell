use std-rfc/path
use std/assert

#[test]
def path_with_extension [] {
    let new_path = "ab.txt" | path with-extension "rs"
    assert equal $new_path "ab.rs"

    let new_path = "ab.txt" | path with-extension ".rs"
    assert equal $new_path "ab.rs"
}

#[test]
def path_with_extension_for_list [] {
    let new_path = ["ab.txt", "cd.exe"] | path with-extension "rs"
    assert equal $new_path ["ab.rs", "cd.rs"]


    let new_path = ["ab.txt", "cd.exe"] | path with-extension ".rs"
    assert equal $new_path ["ab.rs", "cd.rs"]
}

#[test]
def path_with_stem [] {
    let new_path = "/usr/bin" | path with-stem "share"
    assert equal $new_path "/usr/share"

    let new_path = ["/home/alice/", "/home/bob/secret.txt"] | path with-stem "nushell"
    assert equal $new_path ["/home/nushell", "/home/bob/nushell.txt"]
}

#[test]
def path_with_parent [] {
    let new_path = "/etc/foobar" | path with-parent "/usr/share/"
    assert equal $new_path "/usr/share/foobar"

    let new_path = ["/home/rose/meow", "/home/fdncred/"] | path with-parent "/root/"
    assert equal $new_path ["/root/meow", "/root/fdncred"]
}
