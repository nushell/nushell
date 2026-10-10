#!/usr/bin/env nu
# Run every fixture in `tests/fixtures/` through three front ends and report
# where they disagree:
#
#   ours   nu-winnow-parser (the `parse` example, `--check`)
#   nu     the `nu` binary on PATH (`nu-check`, i.e. the released parser)
#   main   nu-parser from the local Nushell checkout, through
#          `tools/nushell-harness` (`nu-parser-check`), when it is built
#
# A fixture's expected verdict is its directory: `accept/` must parse,
# `reject/` must not. Rows where any front end disagrees with the expected
# verdict are printed; `--details` returns the whole table instead.
#
#   cargo build --release --example parse
#   (cd tools/nushell-harness; CARGO_TARGET_DIR=../../../../target cargo build --release --bin nu-parser-check)
#   nu tools/scripts/fixtures-compare.nu
#   nu tools/scripts/fixtures-compare.nu --details | where {|r| $r.nu != $r.expected }

# One verdict per fixture from nu-winnow-parser.
def ours-verdicts [parse: path, files: list<path>]: nothing -> list<bool> {
    $files | each {|f| (^$parse --check --quiet $f | complete | get exit_code) == 0 }
}

# One verdict per fixture from the `nu` binary's own parser.
def nu-verdicts [files: list<path>]: nothing -> list<bool> {
    # `nu-check $f` would check the *input* string when one is piped in, so feed it the source.
    $files | each {|f| open --raw $f | nu-check }
}

# One verdict per fixture from the local checkout's nu-parser, or null when
# the harness binary is not built.
def main-verdicts [check: path, files: list<path>]: nothing -> table<main: any, main_error: any> {
    if not ($check | path exists) {
        return ($files | each { {main: null, main_error: null} })
    }
    let lines = ^$check --quiet ...$files | complete | get stdout | lines
    let errors = $lines
        | where ($it | str starts-with "error ")
        | parse -r '^error (?<file>[^:]+): (?<main_error>.*)'
    $files | each {|f|
        let err = $errors | where file == $f | get -o 0.main_error
        {main: ($err == null), main_error: $err}
    }
}

export def main [
    --parse: path         # the `parse` example binary (default: <workspace target>/release/examples/parse)
    --check: path         # the harness `nu-parser-check` binary (default: <workspace target>/release/nu-parser-check)
    --fixtures: path      # the fixture root (default: tests/fixtures)
    --details             # return the full table instead of printing the disagreements
]: nothing -> any {
    let root = if ("tests/fixtures" | path exists) { $env.PWD } else { $env.FILE_PWD | path join ../.. | path expand }
    # The workspace's target directory: the crate and the harness both build there.
    let target = ($env.CARGO_TARGET_DIR? | default ($env.FILE_PWD | path join ../../../../target) | path expand)
    let parse = $parse | default ($target | path join release/examples/parse)
    let check = $check | default ($target | path join release/nu-parser-check)
    let fixtures = $fixtures | default ($root | path join tests/fixtures)
    let files = glob ($fixtures | path join '**/*.nu') | sort
    let expected = $files | each {|f| ($f | path relative-to $fixtures | path split | first) == "accept" }
    let rows = $files
        | wrap file
        | merge ($expected | wrap expected)
        | merge (ours-verdicts $parse $files | wrap ours)
        | merge (nu-verdicts $files | wrap nu)
        | merge (main-verdicts $check $files)
        | update file {|r| $r.file | path relative-to $fixtures }
    if $details {
        return $rows
    }
    let ours_wrong = $rows | where {|r| $r.ours != $r.expected }
    let nu_wrong = $rows | where {|r| $r.nu != $r.expected }
    let main_wrong = $rows | where {|r| $r.main != null and $r.main != $r.expected }
    let ours_vs_nu = $rows | where {|r| $r.ours != $r.nu }
    print $"($rows | length) fixtures: ours disagrees with the expected verdict on ($ours_wrong | length), nu on ($nu_wrong | length), nushell-main on ($main_wrong | length); ours and nu disagree on ($ours_vs_nu | length)"
    if ($ours_vs_nu | is-not-empty) {
        print "ours and nu disagree:"
        print ($ours_vs_nu | select file expected ours nu main main_error | table -e)
    }
    let nu_only = $nu_wrong | where {|r| $r.ours == $r.expected }
    if ($nu_only | is-not-empty) {
        print "nu disagrees with the expected verdict where ours agrees (usually semantic errors: unknown variables, missing files, types):"
        print ($nu_only | select file expected ours nu main main_error | table -e)
    }
    let main_only = $main_wrong | where {|r| $r.nu == $r.expected }
    if ($main_only | is-not-empty) {
        print "nushell-main disagrees with nu on:"
        print ($main_only | select file expected ours nu main main_error | table -e)
    }
}
