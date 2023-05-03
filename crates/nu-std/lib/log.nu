const CRITICAL_LEVEL = 50
const ERROR_LEVEL = 40
const WARNING_LEVEL = 30
const INFO_LEVEL = 20
const DEBUG_LEVEL = 10

def parse-string-level [level: string] {
    if $level == "CRITICAL" { $CRITICAL_LEVEL }
    else if $level == "CRIT" { $CRITICAL_LEVEL }
    else if $level == "ERROR" { $ERROR_LEVEL }
    else if $level == "WARNING" { $WARNING_LEVEL }
    else if $level == "WARN" { $WARNING_LEVEL }
    else if $level == "INFO" { $INFO_LEVEL }
    else if $level == "DEBUG" { $DEBUG_LEVEL }
    else { $INFO_LEVEL }
}

def current-log-level [] {
    let env_level = ($env | get --ignore-errors NU_LOG_LEVEL | default $INFO_LEVEL)

    try {
        ($env_level | into int)
    } catch {
        parse-string-level $env_level
    }
}

def now [] {
    date now | date format "%Y-%m-%dT%H:%M:%S%.3f"
}

# Log a critical message
export def "log critical" [
    message: string,
    --short (-s)
] {
    if (current-log-level) > $CRITICAL_LEVEL { return }

    let prefix = (if $short { "C" } else { "CRT" })
    print --stderr $"(ansi red_bold)($prefix)|(now)|($message)(ansi reset)"
}

# Log an error message
export def "log error" [
    message: string,
    --short (-s)
] {
    if (current-log-level) > $ERROR_LEVEL { return }

    let prefix = (if $short { "E" } else { "ERR" })
    print --stderr $"(ansi red)($prefix)|(now)|($message)(ansi reset)"
}

# Log a warning message
export def "log warning" [
    message: string,
    --short (-s)
] {
    if (current-log-level) > $WARNING_LEVEL { return }

    let prefix = (if $short { "W" } else { "WRN" })
    print --stderr $"(ansi yellow)($prefix)|(now)|($message)(ansi reset)"
}

# Log an info message
export def "log info" [
    message: string,
    --short (-s)
] {
    if (current-log-level) > $INFO_LEVEL { return }

    let prefix = (if $short { "I" } else { "INF" })
    print --stderr $"(ansi default)($prefix)|(now)|($message)(ansi reset)"
}

# Log a debug message
export def "log debug" [
    message: string,
    --short (-s)
] {
    if (current-log-level) > $DEBUG_LEVEL { return }

    let prefix = (if $short { "D" } else { "DBG" })
    print --stderr $"(ansi default_dimmed)($prefix)|(now)|($message)(ansi reset)"
}
