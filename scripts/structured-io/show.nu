#!/usr/bin/env nu

# Nested-nu structured IO demo. Run this file directly:
#   ./scripts/structured-io/show.nu
#
# Shebang scripts are `nu script.nu`. They honor `NU_EXPERIMENTAL_OPTIONS`
# (so `all` in your environment is enough). If that env is unset, this file
# shows the broken byte-stream case, then re-execs with
# `--experimental-options structured-io=true`.
#
# `print` does the talking. The default `display_output` hook runs `table` on
# whatever `main` returns, and that write hits a broken pipe when this file is
# launched as an external from another nu.

$env.config.hooks.display_output = {|| ignore }

def structured-io-enabled [] {
    let rows = (
        debug experimental-options
        | where identifier == "structured-io"
    )
    if ($rows | is-empty) {
        false
    } else {
        $rows.0.enabled
    }
}

def knows-structured-io [] {
    not (
        debug experimental-options
        | where identifier == "structured-io"
        | is-empty
    )
}

def heading [title: string] {
    print ""
    print $"━━ ($title)"
}

def nu-bin [] {
    $nu.current-exe
}

def script [name: string] {
    $env.FILE_PWD | path join $name
}

def show-broken [exe: string, inventory: string] {
    heading "without structured-io, a .nu script is an external"
    # Collect first so the child can finish writing. Piping a byte stream
    # straight into `columns` drops the pipe and the child dies with
    # broken_pipe from `table` in display_output.
    let nested = (^$exe -n $inventory)
    print "describe of nested output:"
    print ($nested | describe)

    heading "so this is the error people hit"
    try {
        $nested | columns
    } catch {|err|
        print ($err.msg? | default "columns cannot read a byte stream")
    }
}

def show-on [exe: string, inventory: string, healthy: string, summarize: string] {
    heading "the same script, now a real table"
    ^$exe -n $inventory | print

    heading "columns and cell paths work across the subprocess"
    print (^$exe -n $inventory | columns | str join ", ")
    print (^$exe -n $inventory | get host)

    heading "types JSON would smash still exist"
    let sample = ^$exe -n $inventory | get 0
    print $"disk is ($sample.disk | describe): ($sample.disk)"
    print $"uptime is ($sample.uptime | describe): ($sample.uptime)"
    print $"last_patch is ($sample.last_patch | describe)"

    heading "three scripts, one pipeline, no to nuon / from nuon"
    ^$exe -n $inventory | ^$exe -n $healthy | ^$exe -n $summarize | print

    heading "ls in a child nu, then sort like a builtin"
    ls $env.FILE_PWD
    | ^$exe -n -c '$in | where type == file | sort-by size --reverse | update name { path basename } | select name size'
    | print
}

def main [--on] {
    let exe = nu-bin
    let inventory = script "inventory.nu"
    let healthy = script "healthy.nu"
    let summarize = script "summarize.nu"

    if not (knows-structured-io) {
        heading "this nu binary does not have structured-io"
        print $"current exe: ($exe)"
        print "install this checkout or run:"
        print "  cargo run -- --experimental-options structured-io=true scripts/structured-io/show.nu"
        return
    }

    if not (structured-io-enabled) {
        if not $on {
            show-broken $exe $inventory
        }
        heading "relaunching with structured-io"
        ^$exe --experimental-options structured-io=true $env.CURRENT_FILE --on
        return
    }

    show-on $exe $inventory $healthy $summarize
}
