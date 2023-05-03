def CRITICAL_LEVEL [] {
	50
}

def ERROR_LEVEL [] {
	40
}

def WARNING_LEVEL [] {
	30
}

def INFO_LEVEL [] {
	20
}

def DEBUG_LEVEL [] {
	10
}

def parse-string-level [level: string] {
    (
        if $level == "CRITICAL" { CRITICAL_LEVEL }
        else if $level == "CRIT" { CRITICAL_LEVEL }
        else if $level == "ERROR" { ERROR_LEVEL }
        else if $level == "WARNING" { WARNING_LEVEL }
        else if $level == "WARN" { WARNING_LEVEL }
        else if $level == "INFO" { INFO_LEVEL }
        else if $level == "DEBUG" { DEBUG_LEVEL }
        else { (INFO_LEVEL) }
    )
}

def current-log-level [] {
    let env_level = ($env | get --ignore-errors NU_LOG_LEVEL | default (INFO_LEVEL))

    try {
        (env_level | into int)
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
    print --stderr $"($color)($prefix)|(now)|(ansi u)($message)(ansi reset)"
}

# Log a critical message
export def "log critical" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (CRITICAL_LEVEL) { return }

    let prefix = (if $short { "C" } else { "CRT" })
    log-formatted (ansi red_bold) $prefix $message
}

# Log an error message
export def "log error" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (ERROR_LEVEL) { return }

    let prefix = (if $short { "E" } else { "ERR" })
    log-formatted (ansi red) $prefix $message
}

# Log a warning message
export def "log warning" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (WARNING_LEVEL) { return }

    let prefix = (if $short { "W" } else { "WRN" })
    log-formatted (ansi yellow) $prefix $message
}

# Log an info message
export def "log info" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (INFO_LEVEL) { return }

    let prefix = (if $short { "I" } else { "INF" })
    log-formatted (ansi default) $prefix $message
}

# Log a debug message
export def "log debug" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    if (current-log-level) > (DEBUG_LEVEL) { return }

    let prefix = (if $short { "D" } else { "DBG" })
    log-formatted (ansi default_dimmed) $prefix $message
}
