use std/testing *
use std/assert
use std/help

def external-help-echo-helper [] {
    let helper = (mktemp -t --suffix .nu)
    [
        $"#!($nu.current-exe)"
        'def main [...args] { $"ARGS:($args | to nuon)" }'
    ] | str join "\n" | save --force $helper
    chmod +x $helper
    $helper
}

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
    let helper = (external-help-echo-helper)
    let help_code = 'use std/help; help -- test-%.nu'
    let help_result = (
        with-env { NU_HELPER: $helper } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )
    let helper_result = ($help_result | lines | where ($it | str starts-with "ARGS:") | first)

    assert ("test-%.nu" in $helper_result)
    assert ("test-.nu" not-in $helper_result)
}

@test
def external_help_passes_command_words_as_separate_arguments [] {
    let helper = (external-help-echo-helper)
    let help_code = 'use std/help; help git commit'
    let help_result = (
        with-env { NU_HELPER: $helper } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )
    let helper_result = ($help_result | lines | where ($it | str starts-with "ARGS:") | first)

    assert ("git" in $helper_result)
    assert ("commit" in $helper_result)
}

@test
def external_help_split_quoted_command_for_help_flag_helper [] {
    let version = (version).version
    let command = $"($nu.current-exe) --version"
    let help_code = $"use std/help; help \"($command)\""
    let help_result = (
        with-env { NU_HELPER: "--help" } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )

    assert ($version in $help_result)
}
