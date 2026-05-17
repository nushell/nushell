use std-rfc/path
use std/assert
use std/testing *

@test
def path_with_extension [] {
    let new_path = "ab.txt" | path with-extension "rs"
    assert equal $new_path "ab.rs"

    let new_path = "ab.txt" | path with-extension ".rs"
    assert equal $new_path "ab.rs"
}

@test
def path_with_extension_for_list [] {
    let new_path = ["ab.txt", "cd.exe"] | path with-extension "rs"
    assert equal $new_path ["ab.rs", "cd.rs"]

    let new_path = ["ab.txt", "cd.exe"] | path with-extension ".rs"
    assert equal $new_path ["ab.rs", "cd.rs"]
}

@test
def path_with_stem [] {
    let new_path = $"(char psep)usr(char psep)bin" | path with-stem "share"
    assert equal $new_path $"(char psep)usr(char psep)share"

    let new_path = [$"(char psep)home(char psep)alice(char psep)", $"(char psep)home(char psep)bob(char psep)secret.txt"] | path with-stem "nushell"
    assert equal $new_path [$"(char psep)home(char psep)nushell", $"(char psep)home(char psep)bob(char psep)nushell.txt"]
}

@test
def path_with_parent [] {
    let new_path = $"(char psep)etc(char psep)foobar" | path with-parent $"(char psep)usr(char psep)share(char psep)"
    assert equal $new_path $"(char psep)usr(char psep)share(char psep)foobar"

    let new_path = [$"(char psep)home(char psep)rose(char psep)meow", $"(char psep)home(char psep)fdncred(char psep)"] | path with-parent $"(char psep)root(char psep)"
    assert equal $new_path [$"(char psep)root(char psep)meow", $"(char psep)root(char psep)fdncred"]
}
