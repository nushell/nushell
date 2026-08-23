#!/usr/bin/env nu

# Nested-nu structured IO demo. Run this file directly:
#   ./scripts/structured-io/show.nu
#
# Before/after is the child invocation, not this process:
#   $nu.current-exe --experimental-options structured-io=false ...
#   $nu.current-exe --experimental-options structured-io=true ...
# so it works from a REPL that already has `NU_EXPERIMENTAL_OPTIONS=all`.
#
# `print` does the talking. The default `display_output` hook runs `table` on
# whatever `main` returns, and that write hits a broken pipe when this file is
# launched as an external from another nu.

$env.config.hooks.display_output = {|| ignore }

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

# Child argv that turns the handshake off or on, regardless of this process.
def child-nu [on: bool] {
    let flag = if $on { "structured-io=true" } else { "structured-io=false" }
    [--experimental-options $flag -n]
}

def main [] {
    let exe = nu-bin
    let inventory = script "inventory.nu"
    let healthy = script "healthy.nu"
    let summarize = script "summarize.nu"
    let off = child-nu false
    let on = child-nu true

    if not (knows-structured-io) {
        heading "this nu binary does not have structured-io"
        print $"current exe: ($exe)"
        print "install this checkout, then run ./scripts/structured-io/show.nu"
        return
    }

    heading "before: nested nu with structured-io=false"
    # Collect first so the child can finish writing. Piping a byte stream
    # straight into `columns` drops the pipe and the child dies with
    # broken_pipe from `table` in display_output.
    let nested = (^$exe ...$off $inventory)
    print "describe of nested output:"
    print ($nested | describe)

    heading "so this is the error people hit"
    try {
        $nested | columns
    } catch {|err|
        print ($err.msg? | default "columns cannot read a byte stream")
    }

    heading "after: the same script with structured-io=true"
    ^$exe ...$on $inventory | print

    heading "columns and cell paths work across the subprocess"
    print (^$exe ...$on $inventory | columns | str join ", ")
    print (^$exe ...$on $inventory | get host)

    heading "types JSON would smash still exist"
    let sample = (^$exe ...$on $inventory | get 0)
    print $"disk is ($sample.disk | describe): ($sample.disk)"
    print $"uptime is ($sample.uptime | describe): ($sample.uptime)"
    print $"last_patch is ($sample.last_patch | describe)"

    heading "three scripts, one pipeline, no to nuon / from nuon"
    ^$exe ...$on $inventory | ^$exe ...$on $healthy | ^$exe ...$on $summarize | print

    heading "ls in a child nu, then sort like a builtin"
    ls $env.FILE_PWD
    | ^$exe ...$on -c '$in | where type == file | sort-by size --reverse | update name { path basename } | select name size'
    | print
}
