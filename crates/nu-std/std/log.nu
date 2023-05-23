export def CRITICAL_LEVEL [] {
    50
}

export def ERROR_LEVEL [] {
    40
}

export def WARNING_LEVEL [] {
    30
}

export def INFO_LEVEL [] {
    20
}

export def DEBUG_LEVEL [] {
    10
}

def parse-string-level [
    level: string
] {
    if $level in [(CRITICAL_LEVEL_PREFIX) (CRITICAL_LEVEL_PREFIX --short) "CRIT" "CRITICAL"] {
        CRITICAL_LEVEL
    } else if $level in [(ERROR_LEVEL_PREFIX) (ERROR_LEVEL_PREFIX --short) "ERROR" ] {
        ERROR_LEVEL
    } else if $level in [(WARNING_LEVEL_PREFIX) (WARNING_LEVEL_PREFIX --short) "WARN" "WARNING"] {
        WARNING_LEVEL
    } else if $level in [(DEBUG_LEVEL_PREFIX) (DEBUG_LEVEL_PREFIX --short) "DEBUG"] {
        DEBUG_LEVEL
    } else {
        INFO_LEVEL
    }
}

export def CRITICAL_LEVEL_PREFIX [
    --short (-s)
] {
    if $short {
        "C"
    } else {
        "CRT"
    }
}

export def ERROR_LEVEL_PREFIX [
    --short (-s)
] {
    if $short {
        "E"
    } else {
        "ERR"
    }
}

export def WARNING_LEVEL_PREFIX [
    --short (-s)
] {
    if $short {
        "W"
    } else {
        "WRN"
    }
}

export def INFO_LEVEL_PREFIX [
    --short (-s)
] {
    if $short {
        "I"
    } else {
        "INF"
    }
}

export def DEBUG_LEVEL_PREFIX [
    --short (-s)
] {
    if $short {
        "D"
    } else {
        "DBG"
    }
}

def parse-int-level [
    level: int,
    --short (-s)
] {
    if $level >= (CRITICAL_LEVEL) {
        if $short {
            CRITICAL_LEVEL_PREFIX --short
        } else {
            CRITICAL_LEVEL_PREFIX
        }
    } else if $level >= (ERROR_LEVEL) {
        if $short {
            ERROR_LEVEL_PREFIX --short
        } else {
            ERROR_LEVEL_PREFIX
        }
    } else if $level >= (WARNING_LEVEL) {
        if $short {
            WARNING_LEVEL_PREFIX --short
        } else {
            WARNING_LEVEL_PREFIX
        }
    } else if $level >= (INFO_LEVEL) {
        if $short {
            INFO_LEVEL_PREFIX --short
        } else {
            INFO_LEVEL_PREFIX
        }
    } else {
        if $short {
            DEBUG_LEVEL_PREFIX --short
        } else {
            DEBUG_LEVEL_PREFIX
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default (INFO_LEVEL))

    try {
        $env_level | into int
    } catch {
        parse-string-level $env_level
    }
}

def now [] {
    date now | date format "%Y-%m-%dT%H:%M:%S%.3f"
}

def log-formatted [
    color: string,
    prefix: string,
    message: string
] {
    print --stderr $"($color)(now)|($prefix)|(ansi u)($message)(ansi reset)"
}

# Log a critical message
export def critical [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (CRITICAL_LEVEL) {
        return
    }

    let prefix = if $short {
        CRITICAL_LEVEL_PREFIX --short
    } else {
        CRITICAL_LEVEL_PREFIX
    }
    log-formatted (ansi red_bold) $prefix $message
}

# Log an error message
export def error [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (ERROR_LEVEL) {
        return
    }

    let prefix = if $short {
        ERROR_LEVEL_PREFIX --short
    } else {
        ERROR_LEVEL_PREFIX
    }
    log-formatted (ansi red) $prefix $message
}

# Log a warning message
export def warning [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (WARNING_LEVEL) {
        return
    }

    let prefix = if $short {
        WARNING_LEVEL_PREFIX --short
    } else {
        WARNING_LEVEL_PREFIX
    }
    log-formatted (ansi yellow) $prefix $message
}

# Log an info message
export def info [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (INFO_LEVEL) {
        return
    }

    let prefix = if $short {
        INFO_LEVEL_PREFIX --short
    } else {
        INFO_LEVEL_PREFIX
    }
    log-formatted (ansi default) $prefix $message
}

# Log a debug message
export def debug [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (DEBUG_LEVEL) {
        return
    }

    let prefix = if $short {
        DEBUG_LEVEL_PREFIX --short
    } else {
        DEBUG_LEVEL_PREFIX
    }
    log-formatted (ansi default_dimmed) $prefix $message
}

# Log a message with a specific format and verbosity level
# 
# Format reference:
# - %MSG% will be replaced by $message
# - %DATE% will be replaced by the timestamp of log in standard Nushell's log format: "%Y-%m-%dT%H:%M:%S%.3f"
# - %LEVEL% will be replaced by the standard Nushell's log verbosity prefixes, e.g. "CRT"
#
# Examples:
# - std log custom "my message" $"(ansi yellow)[%LEVEL%]MY MESSAGE: %MSG% [%DATE%](ansi reset)" (std log WARNING_LEVEL)
export def custom [
    message: string, # A message
    format: string, # A format
    log_level: int # A log level
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > ($log_level) {
        return
    }

    let level = ((if $short {
        parse-int-level $log_level --short
    } else {
        parse-int-level $log_level
    }) | into string)

    print --stderr ([
        ["%MSG%" $message]
        ["%DATE%" (now)]
        ["%LEVEL%" $level]
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    })
}
