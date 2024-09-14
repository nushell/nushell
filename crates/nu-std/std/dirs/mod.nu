# Maintain a list of working directories and navigate them

# The directory stack.
#
# Exception: the entry for the current directory contains an
# irrelevant value. Instead, the source of truth for the working
# directory is $env.PWD. It has to be this way because cd doesn't
# know about this module.
#
# Example: the following state represents a user-facing directory
# stack of [/a, /var/tmp, /c], and we are currently in /var/tmp .
#
#     PWD = /var/tmp
#     DIRS_POSITION = 1
#     DIRS_LIST = [/a, /b, /c]
#
# This situation could arise if we started with [/a, /b, /c], then
# we changed directories from /b to /var/tmp.
export-env {
    $env.DIRS_POSITION = 0
    $env.DIRS_LIST = [($env.PWD | path expand)]
}

# Add one or more directories to the list.
# PWD becomes first of the newly added directories.
export def --env add [
    ...paths: string    # directory or directories to add to working list
    ] {
        mut abspaths = []
        for p in $paths {
            let exp = ($p | path expand)
            if ($exp | path type) != 'dir' {
                let span = (metadata $p).span
                error make {msg: "not a directory", label: {text: "not a directory", span: $span } }
            }
            $abspaths = ($abspaths | append $exp)
        }

        $env.DIRS_LIST = ($env.DIRS_LIST | insert ($env.DIRS_POSITION + 1) $abspaths | flatten)


    _fetch 1
}

export alias enter = add

# Advance to the next directory in the list or wrap to beginning.
export def --env next [
    N:int = 1   # number of positions to move.
] {
    _fetch $N
}

export alias n = next

# Back up to the previous directory or wrap to the end.
export def --env prev [
    N:int = 1   # number of positions to move.
] {
    _fetch (-1 * $N)
}

export alias p = prev

# Drop the current directory from the list, if it's not the only one.
# PWD becomes the next working directory
export def --env drop [] {
    if ($env.DIRS_LIST | length) > 1 {
        $env.DIRS_LIST = ($env.DIRS_LIST | reject $env.DIRS_POSITION)
        if ($env.DIRS_POSITION >= ($env.DIRS_LIST | length)) {$env.DIRS_POSITION = 0}
    }

    # step to previous slot
    _fetch -1 --forget_current --always_cd

}

export alias dexit = drop

# Display current working directories.
export def --env show [] {
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

export def --env goto [shell?: int] {
    if $shell == null {
        return (show)
    }

    if $shell < 0 or $shell >= ($env.DIRS_LIST | length) {
        let span = (metadata $shell | get span)
        error make {
            msg: $"(ansi red_bold)invalid_shell_index(ansi reset)"
            label: {
                text: $"`shell` should be between 0 and (($env.DIRS_LIST | length) - 1)"
                span: $span
            }
        }
    }

    _fetch ($shell - $env.DIRS_POSITION)
}

export alias g = goto

# fetch item helper
def --env _fetch [
    offset: int,        # signed change to position
    --forget_current    # true to skip saving PWD
    --always_cd         # true to always cd
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
    if ($always_cd or $pos != $env.DIRS_POSITION) {
        $env.DIRS_POSITION = $pos
        cd ($env.DIRS_LIST | get $pos )
    }
}
