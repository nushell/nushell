# std.nu, `used` to load all standard library components

export module help
export module input
export module iter
export module log
export module assert

# Allow use of non-prefixed commands
# from these submodules when `use std *`
export use lib *
export use bench *
export use dt *
export use formats *
export use xml *
export use math *

# Load main dirs command and all subcommands
export use dirs main
export module dirs {
  export use dirs [
    add
    drop
    next
    prev
    goto
  ]
}

# Backward compatibility
# Allow, for example, `formats to jsonl`
export module xml
export module formats
export module dt