# Maintain a list of working directories and navigates them

# the directory stack
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [($env.PWD | path expand)]
}

# Add one or more directories to the list.
# PWD becomes first of the newly added directories.
export def-env "dirs add" [
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

export alias enter = dirs add

# Advance to the next directory in the list or wrap to beginning.
export def-env "dirs next" [
    N:int = 1   # number of positions to move.
] {
    _fetch $N    
}

export alias n = dirs next

# Back up to the previous directory or wrap to the end.
export def-env "dirs prev" [
    N:int = 1   # number of positions to move.
] {
    _fetch (-1 * $N)    
}

export alias p = dirs prev

# Drop the current directory from the list, if it's not the only one.
# PWD becomes the next working directory
export def-env "dirs drop" [] {
    if ($env.DIRS_LIST | length) > 1 {
        let-env DIRS_LIST = (
            ($env.DIRS_LIST | take $env.DIRS_POSITION) 
            | append ($env.DIRS_LIST | skip ($env.DIRS_POSITION + 1))
        )
    }

    _fetch 0
}

export alias dexit = dirs drop

# Display current working directories.
export def-env "dirs show" [] {
    mut out = []
    for $p in ($env.DIRS_LIST | enumerate) {
        $out = ($out | append [
            [active, path]; 
            [($p.index == $env.DIRS_POSITION), $p.item]
        ])
    }

    $out
}

export alias shells = dirs show

export def-env "dirs goto" [shell?: int] {
    if $shell == null {
        return (dirs show)
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

export alias g = dirs goto

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
