# Maintain a ring of working directories and navigate them.

use log *

# the directory ring, wraps around in either direction.
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [($env.PWD | path expand)]
    
    # leaving the following section commented out till some changes in nu startup can allow it to actually work
    ## hotwire user's config to hook PWD changes to notify us
    ##
    ## defining the closure once here seems to guarantee a stable ID no matter how many times the module is use'd
    #let the_hook = {|before, after|
    #    if not ($after | is-empty) {
    #        let-env DIRS_LIST = ($env.DIRS_LIST | update $env.DIRS_POSITION $after)
    #   }
    #}
    #
    #$env.config = ($env.config? | default {})
    #$env.config.hooks = ($env.config.hooks? | default {})
    #$env.config.hooks.env_change = ($env.config.hooks.env_change? | default {})
    #$env.config.hooks.env_change.PWD = ($env.config.hooks.env_change.PWD? | default [])
    #
    #if not ($the_hook in $env.config.hooks.env_change.PWD) { # only add the hook into the list once.
    #    $env.config.hooks.env_change.PWD = ($env.config.hooks.env_change.PWD | append [ $the_hook])
    #}
}

# Add one or more directories to the list.
# PWD becomes first of the newly added directories.
export def-env add [
    ...paths: string    # directory or directories to add to working list
    ] {
        mut abspaths = []
        for p in $paths {
            let exp = ($p | path expand)
            if ($exp | path type) != 'dir' {
                let span = (metadata $p).span
                error make {msg: "not a directory", label: {text: "not a directory", start: $span.start, end: $span.end } }
            }
        $abspaths = ($abspaths | append $exp)

        }
        let-env DIRS_LIST = ($env.DIRS_LIST | insert ($env.DIRS_POSITION + 1) $abspaths | flatten)
        let-env DIRS_POSITION = $env.DIRS_POSITION + 1

    _fetch 0
}

export alias enter = add

# Advance to the next directory in the list or wrap to beginning.
export def-env next [
    N:int = 1   # number of positions to move.
] {
    _fetch $N    
}

export alias n = next

# Back up to the previous directory or wrap to the end.
export def-env prev [
    N:int = 1   # number of positions to move.
] {
    _fetch (-1 * $N)    
}

export alias p = prev

# Drop the current directory from the list, if it's not the only one.
# PWD becomes the next working directory
export def-env drop [] {
    if ($env.DIRS_LIST | length) > 1 {
        let-env DIRS_LIST = (
            ($env.DIRS_LIST | take $env.DIRS_POSITION) 
            | append ($env.DIRS_LIST | skip ($env.DIRS_POSITION + 1))
        )
    }

    _fetch 0
}

export alias dexit = drop

# Display current working directories.
export def-env show [] {
    mut out = []
    for $p in ($env.DIRS_LIST | enumerate) {
        $out = ($out | append [
            [active, path]; 
            [($p.index == $env.DIRS_POSITION), $p.item]
        ])
    }

    $out
}

export alias shells = show

export def-env goto [shell?: int] {
    if $shell == null {
        return (show)
    }

    if $shell < 0 or $shell >= ($env.DIRS_LIST | length) {
        let span = (metadata $shell | get span)
        error make {
            msg: $"(ansi red_bold)invalid_shell_index(ansi reset)"
            label: {
                text: $"`shell` should be between 0 and (($env.DIRS_LIST | length) - 1)"
                start: $span.start
                end: $span.end
            }
        }
    }
    let-env DIRS_POSITION = $shell

    cd ($env.DIRS_LIST | get $env.DIRS_POSITION)
}

export alias g = goto

# PWD change handler to allow dirs to track `cd` changes
# Add this to your config.nu:
#
#   let-env config {
#       . . .
#       hooks: {
#           env_changed: {
#               PWD: [{|before, after| std dirs cdhook $before $after}]
#           }
#       }
#       . . .
#   }
export def-env cdhook [before? after?] {
    if not ($after | is-empty) {
        let-env DIRS_LIST = ($env.DIRS_LIST | update $env.DIRS_POSITION $after)
    }
}


# fetch item helper
def-env  _fetch [
    offset: int,    # signed change to position
] {
    # nushell 'mod' operator is really 'remainder', can return negative values.
    # see: https://stackoverflow.com/questions/13683563/whats-the-difference-between-mod-and-remainder    
    let pos = ($env.DIRS_POSITION 
                + $offset 
                + ($env.DIRS_LIST | length)
            ) mod ($env.DIRS_LIST | length)
    let-env DIRS_POSITION = $pos

    cd ($env.DIRS_LIST | get $pos )
}
