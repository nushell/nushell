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

# Workaround for #13403 to load export-env blocks from submodules
export-env {
    # log
    $env.NU_LOG_FORMAT = $env.NU_LOG_FORMAT? | default "%ANSI_START%%DATE%|%LEVEL%|%MSG%%ANSI_STOP%"
    $env.NU_LOG_DATE_FORMAT = $env.NU_LOG_DATE_FORMAT? | default "%Y-%m-%dT%H:%M:%S%.3f"
    
    # dirs
    $env.DIRS_POSITION = 0
    $env.DIRS_LIST = [($env.PWD | path expand)]
}
