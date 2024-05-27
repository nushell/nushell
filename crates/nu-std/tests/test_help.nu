use std assert
use std help

#[test]
def show_help_on_commands [] {
    let help_result = (help alias)
    assert ("item not found" not-in $help_result)
}

