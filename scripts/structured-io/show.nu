#!/usr/bin/env nu

# Nested-nu structured IO demo. Run this file directly:
#   ./scripts/structured-io/show.nu
#
# Shebang children (`./inventory.nu`) are spawned as-is. If the parent process
# is nu, the child infers structured IO. `--structured-io=false` is the off switch.
#
# `print` does the talking. The default `display_output` hook runs `table` on
# whatever `main` returns, and that write hits a broken pipe when this file is
# launched as an external from another nu.

$env.config.hooks.display_output = {|| ignore }

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

def main [] {
    let exe = nu-bin
    let inventory = script "inventory.nu"
    let healthy = script "healthy.nu"
    let summarize = script "summarize.nu"

    heading "before: nested nu with --structured-io=false"
    # Collect first so the child can finish writing. Piping a byte stream
    # straight into `columns` drops the pipe and the child dies with
    # broken_pipe from `table` in display_output.
    let nested = (^$exe --structured-io=false -n $inventory)
    print "describe of nested output:"
    print ($nested | describe)

    heading "so this is the error people hit"
    try {
        $nested | columns
    } catch {|err|
        print ($err.msg? | default "columns cannot read a byte stream")
    }

    heading "after: shebang ./inventory.nu (parent is nu, child infers)"
    ^$inventory | print

    heading "columns and cell paths work across the subprocess"
    print (^$inventory | columns | str join ", ")
    print (^$inventory | get host)

    heading "types JSON would smash still exist"
    let sample = (^$inventory | get 0)
    print $"disk is ($sample.disk | describe): ($sample.disk)"
    print $"uptime is ($sample.uptime | describe): ($sample.uptime)"
    print $"last_patch is ($sample.last_patch | describe)"

    heading "three shebang scripts, one pipeline, no to nuon / from nuon"
    ^$inventory | ^$healthy | ^$summarize | print

    heading "ls in a child nu, then sort like a builtin"
    ls $env.FILE_PWD
    | ^$exe -n -c '$in | where type == file | sort-by size --reverse | update name { path basename } | select name size'
    | print
}
