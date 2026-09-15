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
def show_help_on_external_command_with_sigil_in_name [] {
    # `^` and `%` are sigils only at the start of a command name, not inside it
    let dir = ($nu.temp-dir | path join $"test_help_(random uuid)")
    mkdir $dir
    let script = ($dir | path join "ext-%^-cmd.nu")
    "print 'external help ok'" | save $script

    let help_result = (with-env {NU_HELPER: $nu.current-exe} { help $script })
    rm -r $dir

    assert ("external help ok" in $help_result)
}

@test
def show_help_on_external_command_passes_words_separately [] {
    # `help foo bar` has to reach the helper as two arguments, not as `foo bar`
    let dir = ($nu.temp-dir | path join $"test_help_(random uuid)")
    mkdir $dir
    let script = ($dir | path join "ext-cmd.nu")
    "def main [word: string] { print $'external help ($word)' }" | save $script

    let help_result = (with-env {NU_HELPER: $nu.current-exe} { help $script config })
    rm -r $dir

    assert ("external help config" in $help_result)
}
