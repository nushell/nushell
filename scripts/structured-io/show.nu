#!/usr/bin/env nu

# Nested-nu structured IO demo.
#
# Off (byte stream, columns fail):
#   nu scripts/structured-io/show.nu
#
# On (tables survive the process boundary):
#   nu --experimental-options structured-io=true scripts/structured-io/show.nu

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

    if not (structured-io-enabled) {
        heading "without structured-io, a .nu script is an external"
        print "describe of nested output:"
        print (^$exe -n $inventory | describe)

        heading "so this is the error people hit"
        try {
            ^$exe -n $inventory | columns
        } catch {|err|
            print ($err.msg? | default "columns cannot read a byte stream")
        }

        heading "turn it on and run this file again"
        print $"($exe) --experimental-options structured-io=true ($env.CURRENT_FILE)"
        return (ignore)
    }

    heading "the same script, now a real table"
    ^$exe -n $inventory

    heading "columns and cell paths work across the subprocess"
    print (^$exe -n $inventory | columns | str join ", ")
    print (^$exe -n $inventory | get host)

    heading "types JSON would smash still exist"
    let sample = ^$exe -n $inventory | get 0
    print $"disk is ($sample.disk | describe): ($sample.disk)"
    print $"uptime is ($sample.uptime | describe): ($sample.uptime)"
    print $"last_patch is ($sample.last_patch | describe)"

    heading "three scripts, one pipeline, no to nuon / from nuon"
    ^$exe -n $inventory | ^$exe -n $healthy | ^$exe -n $summarize

    heading "ls in a child nu, then sort like a builtin"
    ls $env.FILE_PWD
    | ^$exe -n -c '$in | where type == file | sort-by size --reverse | update name { path basename } | select name size'
}
