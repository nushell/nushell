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

@test
def external_help_preserves_sigil_characters_inside_command_name [] {
    let help_code = 'use crates/nu-std/std/help; help -- --version test-%.nu'
    let help_result = (
        with-env { NU_HELPER: $nu.current-exe } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )

    assert ("test-%.nu" in $help_result)
    assert ("test-.nu" not-in $help_result)
}

@test
def external_help_passes_command_words_as_separate_arguments [] {
    let version = (open Cargo.toml | get workspace.package.version)
    let help_code = 'use crates/nu-std/std/help; help -- --version ignored'
    let help_result = (
        with-env { NU_HELPER: $nu.current-exe } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )

    assert ($version in $help_result)
}
