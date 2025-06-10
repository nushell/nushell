use std/testing *
use std/assert
use std/help

@test
def show_help_on_commands [] {
    let help_result = (help alias)
    assert ("item not found" not-in $help_result)
}

@test
def show_help_on_error_make [] {
    let help_result = (help error make)
    assert ("Error: nu::shell::eval_block_with_input" not-in $help_result)
}
