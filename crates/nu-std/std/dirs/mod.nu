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
# The first directory listed becomes the new
# active directory.
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

# Make the next directory on the list the active directory.
# If the current active directory is the last in the list,
# then cycle to the top of the list.
export def --env next [
    N:int = 1   # number of positions to move.
] {
    _fetch $N
}

# Make the previous directory on the list the active directory.
# If the current active directory is the first in the list,
# then cycle to the end of the list.
export def --env prev [
    N:int = 1   # number of positions to move.
] {
    _fetch (-1 * $N)
}

# Drop the current directory from the list.
# The previous directory in the list becomes
# the new active directory.
#
# If there is only one directory in the list,
# then this command has no effect.
export def --env drop [] {
    if ($env.DIRS_LIST | length) > 1 {
        $env.DIRS_LIST = ($env.DIRS_LIST | reject $env.DIRS_POSITION)
        if ($env.DIRS_POSITION >= ($env.DIRS_LIST | length)) {$env.DIRS_POSITION = 0}
    }

    # step to previous slot
    _fetch -1 --forget_current --always_cd

}

# Display current working directories
export def --env main [] {
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

# Jump to directory by index
export def --env goto [dir_idx?: int] {
    if $dir_idx == null {
        return (main)
    }

    if $dir_idx < 0 or $dir_idx >= ($env.DIRS_LIST | length) {
        let span = (metadata $dir_idx | get span)
        error make {
            msg: $"(ansi red_bold)invalid_dirs_index(ansi reset)"
            label: {
                text: $"`idx` should be between 0 and (($env.DIRS_LIST | length) - 1)"
                span: $span
            }
        }
    }

    _fetch ($dir_idx - $env.DIRS_POSITION)
}

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

export module shells-aliases {
    export alias shells = main
    export alias enter = add
    export alias dexit = drop
    export alias p = prev
    export alias n = next
    export alias g = goto
}