# std.nu, `use`d to load all standard library components

# Top-level commands: ellie, repeat, null-device, and "path add"
export use std/util *

# std submodules
export module std/assert
export module std/bench
export module std/dt
export module std/formats
export module std/help
export module std/input
export module std/iter
export module std/log
export module std/math
export module std/xml
export module std/config
export module std/testing
export module std/random

# Load main dirs command and all subcommands
export use std/dirs main
export module dirs {
  export use std/dirs [
    add
    drop
    next
    prev
    goto
  ]
}

# Workaround for #13403 to load export-env blocks from submodules
export-env {
    use std/log []
    use std/dirs []
}
