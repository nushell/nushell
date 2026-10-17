use std/testing *
use std/assert
use std/help

# Script body used as NU_HELPER's target. `$nu.current-exe` runs it, so the
# arguments never go through cmd.exe (which rejects `%` on Windows).
def external-help-echo-script [] {
    'def main [...args] { $"ARGS:([($env.CURRENT_FILE | path basename) ...$args] | to nuon)" }'
}

@test
def show_help_on_commands [] {
    let help_result = (help alias)
    assert ("item not found" not-in $help_result)
}

@test
def module_page_lists_exported_type_aliases [] {
    let help_code = "module geo { type Private = int; export type Point = int; export type Id = int }; use std/help; help modules geo | ansi strip"
    let help_result = (^$nu.current-exe --no-config-file --commands $help_code)

    assert ("Exported type aliases:\n    Id\n    Point" in $help_result)
    assert ("Private" not-in $help_result)
}

@test
def show_help_on_error_make [] {
    let help_result = (help error make)
    assert ("Error: nu::shell::" not-in $help_result)
}

@test
def external_help_preserves_sigil_characters_inside_command_name [] {
    let dir = (mktemp -d)
    let name = "test-%.nu"
    external-help-echo-script | save --force ($dir | path join $name)
    let help_code = $"cd ($dir | to nuon); use std/help; help ($name | to nuon)"
    let help_result = (
        with-env { NU_HELPER: $nu.current-exe } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )
    let helper_result = ($help_result | lines | where ($it | str starts-with "ARGS:") | first)

    assert ("test-%.nu" in $helper_result)
    assert ("test-.nu" not-in $helper_result)
}

@test
def external_help_passes_command_words_as_separate_arguments [] {
    let dir = (mktemp -d)
    external-help-echo-script | save --force ($dir | path join "git")
    let help_code = $"cd ($dir | to nuon); use std/help; help git commit"
    let help_result = (
        with-env { NU_HELPER: $nu.current-exe } {
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
    let help_code = $"use std/help; help ($command | to nuon)"
    let help_result = (
        with-env { NU_HELPER: "--help" } {
            ^$nu.current-exe --no-config-file --commands $help_code
        }
    )

    assert ($version in $help_result)
}
