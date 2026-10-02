#!/usr/bin/env nu
# Run the `nufmt` example over the ground-truth fixtures of nushell/nufmt
# (tests/fixtures/input/*.nu formatted, compared with tests/fixtures/expected)
# and report which expected outputs it reproduces.
#
# Like nufmt's own harness, outputs are compared after trimming surrounding
# whitespace. Fixtures with a tests/fixtures/config/<name>.nuon file need
# non-default settings (line length, margin, tabs) and are flagged.
#
# Requires the example binary: `cargo build --release --example nufmt`.
#
#   nu tools/scripts/nufmt-fixtures.nu                  # summary + non-matching names
#   nu tools/scripts/nufmt-fixtures.nu --details        # full table
#   nu tools/scripts/nufmt-fixtures.nu --diff closure   # diff for one fixture

# `--config FILE` when the fixture has a config file, else nothing.
def config-args [fixtures: string, name: string]: nothing -> list<string> {
    let config = $fixtures | path join config $"($name).nuon"
    if ($config | path exists) { ["--config" $config] } else { [] }
}

def main [
    nufmt_dir: string = "~/src/nufmt"  # checkout of https://github.com/nushell/nufmt
    --details                          # return the full table instead of a summary
    --diff: string                     # print `diff <ours> <expected>` for this fixture name
] {
    let bin = ($env.CARGO_TARGET_DIR? | default ($env.FILE_PWD | path join ../../../../target) | path expand) | path join release/examples/nufmt
    let fixtures = $nufmt_dir | path expand | path join tests fixtures
    if not ($fixtures | path exists) {
        error make {msg: $"no fixtures at ($fixtures)"}
    }
    if $diff != null {
        let input = $fixtures | path join input $"($diff).nu"
        let ours = mktemp -t nufmt-ours.XXXXXX
        ^$bin ...(config-args $fixtures $diff) $input | save -f $ours
        ^diff $ours ($fixtures | path join expected $"($diff).nu")
        rm $ours
        return
    }
    let results = glob ($fixtures | path join input *.nu) | sort | each {|input|
        let name = $input | path parse | get stem
        let expected = $fixtures | path join expected $"($name).nu" | open --raw | str trim
        let run = do { ^$bin ...(config-args $fixtures $name) $input } | complete
        let status = if $run.exit_code != 0 {
            "error"
        } else if ($run.stdout | str trim) == $expected {
            "match"
        } else {
            "differ"
        }
        {
            fixture: $name
            status: $status
            config: ($fixtures | path join config $"($name).nuon" | path exists)
            stderr: ($run.stderr | lines | first | default "")
        }
    }
    if $details {
        return $results
    }
    print ($results | group-by status --to-table | update items {|g| $g.items | length } | rename status count)
    $results | where status != "match" | select fixture status config stderr
}
