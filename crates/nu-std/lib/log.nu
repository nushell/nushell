export def "log CRITICAL_LEVEL" [] {
    50
}

export def "log ERROR_LEVEL" [] {
    40
}

export def "log WARNING_LEVEL" [] {
    30
}

export def "log INFO_LEVEL" [] {
    20
}

export def "log DEBUG_LEVEL" [] {
    10
}

def parse-string-level [
    level: string
] {
    if $level in [(log CRITICAL_LEVEL_PREFIX) (log CRITICAL_LEVEL_PREFIX --short) "CRIT" "CRITICAL"] {
        log CRITICAL_LEVEL
    } else if $level in [(log ERROR_LEVEL_PREFIX) (log ERROR_LEVEL_PREFIX --short) "ERROR" ] {
        log ERROR_LEVEL
    } else if $level in [(log WARNING_LEVEL_PREFIX) (log WARNING_LEVEL_PREFIX --short) "WARN" "WARNING"] {
        log WARNING_LEVEL
    } else if $level in [(log DEBUG_LEVEL_PREFIX) (log DEBUG_LEVEL_PREFIX --short) "DEBUG"] {
        log DEBUG_LEVEL
    } else {
        log INFO_LEVEL
    }
}

export def "log CRITICAL_LEVEL_PREFIX" [
    --short (-s)
] {
    if $short {
        "C"
    } else {
        "CRT"
    }
}

export def "log ERROR_LEVEL_PREFIX" [
    --short (-s)
] {
    if $short {
        "E"
    } else {
        "ERR"
    }
}

export def "log WARNING_LEVEL_PREFIX" [
    --short (-s)
] {
    if $short {
        "W"
    } else {
        "WRN"
    }
}

export def "log INFO_LEVEL_PREFIX" [
    --short (-s)
] {
    if $short {
        "I"
    } else {
        "INF"
    }
}

export def "log DEBUG_LEVEL_PREFIX" [
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
    if $level >= (log CRITICAL_LEVEL) {
        if $short {
            log CRITICAL_LEVEL_PREFIX --short
        } else {
            log CRITICAL_LEVEL_PREFIX
        }
    } else if $level >= (log ERROR_LEVEL) {
        if $short {
            log ERROR_LEVEL_PREFIX --short
        } else {
            log ERROR_LEVEL_PREFIX
        }
    } else if $level >= (log WARNING_LEVEL) {
        if $short {
            log WARNING_LEVEL_PREFIX --short
        } else {
            log WARNING_LEVEL_PREFIX
        }
    } else if $level >= (log INFO_LEVEL) {
        if $short {
            log INFO_LEVEL_PREFIX --short
        } else {
            log INFO_LEVEL_PREFIX
        }
    } else {
        if $short {
            log DEBUG_LEVEL_PREFIX --short
        } else {
            log DEBUG_LEVEL_PREFIX
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default (log INFO_LEVEL))

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
export def "log critical" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (log CRITICAL_LEVEL) {
        return
    }

    let prefix = if $short {
        log CRITICAL_LEVEL_PREFIX --short
    } else {
        log CRITICAL_LEVEL_PREFIX
    }
    log-formatted (ansi red_bold) $prefix $message
}

# Log an error message
export def "log error" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (log ERROR_LEVEL) {
        return
    }

    let prefix = if $short {
        log ERROR_LEVEL_PREFIX --short
    } else {
        log ERROR_LEVEL_PREFIX
    }
    log-formatted (ansi red) $prefix $message
}

# Log a warning message
export def "log warning" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (log WARNING_LEVEL) {
        return
    }

    let prefix = if $short {
        log WARNING_LEVEL_PREFIX --short
    } else {
        log WARNING_LEVEL_PREFIX
    }
    log-formatted (ansi yellow) $prefix $message
}

# Log an info message
export def "log info" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (log INFO_LEVEL) {
        return
    }

    let prefix = if $short {
        log INFO_LEVEL_PREFIX --short
    } else {
        log INFO_LEVEL_PREFIX
    }
    log-formatted (ansi default) $prefix $message
}

# Log a debug message
export def "log debug" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (log DEBUG_LEVEL) {
        return
    }

    let prefix = if $short {
        log DEBUG_LEVEL_PREFIX --short
    } else {
        log DEBUG_LEVEL_PREFIX
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
export def "log custom" [
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
