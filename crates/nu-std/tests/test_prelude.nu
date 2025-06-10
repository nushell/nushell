use std/testing *
use std/assert

@test
def banner [] {
    use std/prelude
    assert ((prelude banner | lines | length) == 16)
}
