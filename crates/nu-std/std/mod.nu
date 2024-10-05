# std.nu, `used` to load all standard library components

# Top-level commands: ellie, repeat, null-device, and "path add"
export use lib *

# std submodules
export module ./assert
export module ./bench
export module ./dt
export module ./formats
export module ./help
export module ./input
export module ./iter
export module ./log
export module ./math
export module ./xml

# Load main dirs command and all subcommands
export use ./dirs main
export module ./dirs {
  export use ./dirs [
    add
    drop
    next
    prev
    goto
  ]
}
