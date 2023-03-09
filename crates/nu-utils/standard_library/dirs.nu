# module dirs.nu -- maintain list of remembered directories + navigate them
#
# todo:
# * expand relative to absolute paths (or relative to some prefix?)
# * what if user does `cd` by hand?
# * fix dirs show to use active: true/false


# the directory stack
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [$env.PWD]
}

# Add one or more directories to the list.
# PWD becomes first of the newly added directories.
export def-env "add" [
    ...paths: string    # directory or directories to add to remembered list
    ] {
    let-env DIRS_LIST = ($env.DIRS_LIST | insert ($env.DIRS_POSITION + 1) $paths | flatten)
    let-env DIRS_POSITION = $env.DIRS_POSITION + 1

    _fetch 0
}

# Advance to the next directory in the list or wrap to beginning.
export def-env "next" [
    N:int = 1 # number of positions to move.
] {
    _fetch $N    
}

# Back up to the previous directory or wrap to the end.
export def-env "prev" [
    N:int = 1 # number of positions to move.
] {
    _fetch (-1 * $N)    
}

# Drop the current directory from the list, if it's not the only one.
# PWD becomes the next remembered directory
export def-env "drop" [] {
    if ($env.DIRS_LIST | length) > 1 {
        let-env DIRS_LIST = (($env.DIRS_LIST | take $env.DIRS_POSITION) | append ($env.DIRS_LIST | skip ($env.DIRS_POSITION + 1)))
    }

    _fetch 0
}

# display current remembered directories
export def-env "show" [] {
    mut out = []
    for $p in ($env.DIRS_LIST | enumerate) {
        $out = ($out | append [
            [current, path]; 
            [(if ($p.index == $env.DIRS_POSITION) {
                    "==>"
                } else {
                    ""
                })
            , $p.item]
        ])
    }

    $out
}



# fetch item helper
def-env  _fetch [
    offset: int,            # signed change to position
] {
    # nushell 'mod' operator is really 'remainder', can return negative values.
    # see: https://stackoverflow.com/questions/13683563/whats-the-difference-between-mod-and-remainder    
    let pos = ($env.DIRS_POSITION + $offset + ($env.DIRS_LIST | length)) mod ($env.DIRS_LIST | length)
    # echo $"at ($pos); item is: ($env.DIRS_LIST |  get $pos)"
    let-env DIRS_POSITION = $pos

    cd ($env.DIRS_LIST | get $pos )
}
            