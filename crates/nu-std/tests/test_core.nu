use std/assert

#[test]
def banner [] {
    use std/core
    assert ((core banner | lines | length) == 16)
}
