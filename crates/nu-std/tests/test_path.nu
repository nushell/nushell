use std *
use std/assert

#[test]
def path_extension [] {
    let new_path = "ab.txt" | path extension "rs"
    assert equal $new_path "ab.rs"

    let new_path = "ab.txt" | path extension ".rs"
    assert equal $new_path "ab.rs"
}

#[test]
def path_extension_for_list [] {
    let new_path = ["ab.txt", "cd.exe"] | path extension "rs"
    assert equal $new_path ["ab.rs", "cd.rs"]


    let new_path = ["ab.txt", "cd.exe"] | path extension ".rs"
    assert equal $new_path ["ab.rs", "cd.rs"]
}
