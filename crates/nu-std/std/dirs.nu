# Maintain a list of working directories and navigate them

# the directory stack
# current slot is DIRS_POSITION, but that entry doesn't hold $PWD (until leaving it for some other)
# till then, we let CD change PWD freely
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [($env.PWD | path expand)]
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


    _fetch 1
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
        let-env DIRS_LIST = ($env.DIRS_LIST | reject $env.DIRS_POSITION)
        if ($env.DIRS_POSITION >= ($env.DIRS_LIST | length)) {$env.DIRS_POSITION = 0}
    }

    _fetch -1 --forget_current   # step to previous slot

}

export alias dexit = drop

# Display current working directories.
export def-env show [] {
    mut out = []
    for $p in ($env.DIRS_LIST | enumerate) {
        let is_act_slot = $p.index == $env.DIRS_POSITION
        $out = ($out | append [
            [active, path]; 
            [($is_act_slot), 
            (if $is_act_slot {$env.PWD} else {$p.item})   # show current PWD in lieu of active slot
            ]
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

# fetch item helper
def-env  _fetch [
    offset: int,        # signed change to position
    --forget_current    # true to skip saving PWD
] {
    if not ($forget_current) {
        # first record current working dir in current slot of ring, to track what CD may have done.
        $env.DIRS_LIST = ($env.DIRS_LIST | upsert $env.DIRS_POSITION $env.PWD)
    }

    # figure out which entry to move to
    # nushell 'mod' operator is really 'remainder', can return negative values.
    # see: https://stackoverflow.com/questions/13683563/whats-the-difference-between-mod-and-remainder    
    let len = ($env.DIRS_LIST | length)
    mut pos = ($env.DIRS_POSITION + $offset) mod $len
    if ($pos < 0) { $pos += $len}

    # if using a different position in ring, CD there.
    if ($pos != $env.DIRS_POSITION) {
        $env.DIRS_POSITION = $pos
        cd ($env.DIRS_LIST | get $pos )
    }
}
