use std/util *

#[test]
def repeat_things [] {
    use std/assert
    assert error { "foo" | repeat -1 }

    for x in ["foo", [1 2], {a: 1}] {
        assert equal ($x | repeat 0) []
        assert equal ($x | repeat 1) [$x]
        assert equal ($x | repeat 2) [$x $x]
    }
}
