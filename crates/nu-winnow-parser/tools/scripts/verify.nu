#!/usr/bin/env nu
# The whole verification ladder in one command, with a scoreboard.
#
# Every rung compares this parser with Nushell in a different way; a number
# that moves is a parser change to look at. Run it before a check-in and
# whenever the Nushell checkout moves:
#
#   nu tools/scripts/verify.nu                 # build, run everything, print the scoreboard
#   nu tools/scripts/verify.nu --no-build      # reuse the existing binaries
#   nu tools/scripts/verify.nu --quick         # skip the slow corpora (nu_scripts, mutants)
#   nu tools/scripts/verify.nu --save FILE     # append the scoreboard to a NUON history file
#
# Rungs (and what "good" looks like):
#   cargo test              unit, syntax, fixture, language, example, corpus, traceability, nufmt: 0 failures
#   fixtures-compare        ours vs nu-check vs nu-parser over tests/fixtures: 0 rows where ours != expected
#   command table           src/builtin_commands.rs lists exactly the commands of the harness's engine
#   differential            nu-parser vs ours, in process, over fixtures, corpora, the command examples
#                           and the book, and 3 mutants of each: 0 ours_rejects, 0 nu_syntax_rejects, 0 panics.
#                           No difference is tolerated silently: `differential --details` prints every disagreement
#   nucheck-compare         nu-check vs ours over nu_scripts and nu-std: 0 files nu accepts that we reject
#   flatcmp                 nu's `ast --flatten` vs ours over nu-std: differences are the documented ones
#   nufmt-fixtures          the formatter reproduces the nushell/nufmt reference fixtures: 114 of 130 today
#   traceability            every nu-parser construct is mapped (part of cargo test, listed for visibility)

# Run a command line, returning {ok, stdout} without failing the script.
def --wrapped run-capture [...cmd: string]: nothing -> record<ok: bool, stdout: string> {
    let result = ^$cmd.0 ...($cmd | skip 1) | complete
    {ok: ($result.exit_code == 0), stdout: $result.stdout}
}

export def main [
    --no-build           # do not rebuild the release binaries and the harness
    --quick              # skip nu_scripts, the book and mutation fuzzing
    --save: path         # append the scoreboard to this NUON list
    --nushell: path      # the Nushell checkout (default: the enclosing workspace)
    --nu-scripts: path   # a nu_scripts checkout (default ~/src/nu_scripts)
]: nothing -> table<rung: string, metric: string, value: any, ok: bool> {
    let root = if ("tests/fixtures" | path exists) { $env.PWD } else { $env.FILE_PWD | path join ../.. | path expand }
    cd $root
    let nushell = $nushell | default ($root | path join ../.. | path expand)
    let nu_scripts = $nu_scripts | default ("~/src/nu_scripts" | path expand)
    let harness = $root | path join tools/nushell-harness
    # The workspace's target directory: the crate and the harness both build there.
    let target = $env.CARGO_TARGET_DIR? | default ($root | path join ../../target) | path expand
    let parse = $target | path join release/examples/parse
    let differential = $target | path join release/differential

    if not $no_build {
        print "building the parse and nufmt examples and the harness..."
        cargo build --release --example parse --example nufmt
        cd $harness
        with-env {CARGO_TARGET_DIR: $target} {
            cargo build --release --bin differential --bin nu-parser-check --bin builtin-commands
        }
        cd $root
    }

    mut rows = []

    print "command table..."
    let table = run-capture nu tools/scripts/gen-builtin-commands.nu --check
    $rows ++= [{rung: "command table", metric: "matches the harness engine", value: $table.ok, ok: $table.ok}]

    print "cargo test..."
    let tests = run-capture cargo test --all-features
    # `append 0`: a build that fails prints no `test result:` line at all.
    let failed = $tests.stdout | parse -r 'test result: (?P<status>\w+)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed' | get failed | each { into int } | append 0 | math sum
    let passed = $tests.stdout | parse -r 'test result: (?P<status>\w+)\. (?P<passed>\d+) passed' | get passed | each { into int } | append 0 | math sum
    # cargo's exit status also catches what the summaries miss: a build error, or a test
    # binary that died (a stack overflow) before printing its summary.
    let ok = $tests.ok and $failed == 0
    $rows ++= [{rung: "cargo test", metric: "tests passed", value: $passed, ok: $ok}]
    $rows ++= [{rung: "cargo test", metric: "tests failed", value: $failed, ok: $ok}]

    print "fixtures-compare..."
    use fixtures-compare.nu
    let compare = fixtures-compare --details --parse $parse
    let ours_vs_nu = $compare | where {|r| $r.ours != $r.nu } | length
    let ours_vs_expected = $compare | where {|r| $r.ours != $r.expected } | length
    $rows ++= [{rung: "fixtures-compare", metric: "fixtures", value: ($compare | length), ok: true}]
    # `nu-check` also rejects for semantic reasons (missing module files, plugins), so this is informational;
    # the differential rung below separates syntax from semantics with nu-parser's messages.
    $rows ++= [{rung: "fixtures-compare", metric: "ours != nu (incl. semantic)", value: $ours_vs_nu, ok: true}]
    $rows ++= [{rung: "fixtures-compare", metric: "ours != expected", value: $ours_vs_expected, ok: ($ours_vs_expected == 0)}]

    print "differential..."
    let snippets = [tests/corpus/snippets/nu-command-examples.json] | append (if $quick { [] } else { [tests/corpus/snippets/book.json] })
    let inputs = [tests/fixtures tests/corpus ($nushell | path join crates/nu-std) ($nushell | path join tests)]
        | append (if $quick { [] } else { [$nu_scripts] })
        | where ($it | path exists)
    let snippet_args = $snippets | each {|s| ["--snippets" $s] } | flatten
    let mutant_args = if $quick { [] } else { ["--mutants" "3" "--seed" "1"] }
    let diff = run-capture $differential --json ...$snippet_args ...$mutant_args ...$inputs
    let summary = $diff.stdout | from json
    for set in [originals mutants] {
        let s = $summary | get $set
        if $s.total > 0 {
            $rows ++= [{rung: $"differential ($set)", metric: "compared", value: $s.total, ok: true}]
            $rows ++= [{rung: $"differential ($set)", metric: "ours rejects, nu accepts", value: $s.ours_rejects, ok: ($s.ours_rejects == 0)}]
            $rows ++= [{rung: $"differential ($set)", metric: "nu rejects (syntax), ours accepts", value: $s.nu_syntax_rejects, ok: ($s.nu_syntax_rejects == 0)}]
            $rows ++= [{rung: $"differential ($set)", metric: "nu rejects (semantic)", value: $s.nu_semantic_rejects, ok: true}]
            $rows ++= [{rung: $"differential ($set)", metric: "panics", value: $s.panics, ok: ($s.panics == 0)}]
        }
    }

    print "nucheck-compare..."
    use nucheck-compare.nu
    let dirs = [($nushell | path join crates/nu-std)] | append (if $quick { [] } else { [$nu_scripts] }) | where ($it | path exists)
    let verdicts = nucheck-compare --details --parse $parse ...$dirs
    let bugs = $verdicts | where {|r| $r.accepted_by_nu and not $r.accepted_by_ours } | length
    let lenient = $verdicts | where {|r| $r.accepted_by_ours and not $r.accepted_by_nu } | length
    $rows ++= [{rung: "nucheck-compare", metric: "files", value: ($verdicts | length), ok: true}]
    $rows ++= [{rung: "nucheck-compare", metric: "nu accepts, ours rejects", value: $bugs, ok: ($bugs == 0)}]
    $rows ++= [{rung: "nucheck-compare", metric: "ours accepts, nu rejects (semantic)", value: $lenient, ok: true}]

    print "flatcmp..."
    let std_files = glob ($nushell | path join crates/nu-std '**/*.nu')
    let flat = run-capture nu tools/scripts/flatcmp.nu --commands tools/scripts/std_commands.txt ...$std_files
    let flat_summary = $flat.stdout | lines | last | parse -r '(?P<files>\d+) files, (?P<with>\d+) with differences, (?P<runs>\d+) differing runs'
    let runs = $flat_summary | get -o 0.runs | default "?" | into int
    $rows ++= [{rung: "flatcmp (nu-std)", metric: "differing runs", value: $runs, ok: true}]

    print "nufmt-fixtures..."
    let fmt = run-capture nu tools/scripts/nufmt-fixtures.nu
    let matched = $fmt.stdout | ansi strip | parse -r 'match\s*│\s*(?P<n>\d+)' | get -o 0.n | default "?"
    $rows ++= [{rung: "nufmt-fixtures", metric: "reference fixtures matched", value: $matched, ok: true}]

    if $save != null {
        let history = if ($save | path exists) { open $save } else { [] }
        # The checkout the corpora, the traceability test and the harness (path dependencies) all come from.
        let checkout = git -C $nushell rev-parse --short HEAD | complete | get stdout | str trim
        $history | append {date: (date now | format date '%Y-%m-%d %H:%M'), nushell: $checkout, harness_nushell: $checkout, rows: $rows} | to nuon | save -f $save
    }
    $rows
}
