use std/assert
use std/testing *
use std-rfc/random

@test
def random_choice_return_type_depending_on_n [] {
    let no_n = [a b c] | random choice | describe
    assert equal $no_n "string"

    let with_n = [a b c] | random choice 1 | describe
    assert equal $with_n "list<string>"
}
