export def "log CRITICAL_LEVEL" [] { 50 }
export def "log ERROR_LEVEL"    [] { 40 }
export def "log WARNING_LEVEL"  [] { 30 }
export def "log INFO_LEVEL"     [] { 20 }
export def "log DEBUG_LEVEL"    [] { 10 }

export def "log CRITICAL_LEVEL_PREFIX" [] { "CRT" }
export def "log ERROR_LEVEL_PREFIX"    [] { "ERR" }
export def "log WARNING_LEVEL_PREFIX"  [] { "WRN" }
export def "log INFO_LEVEL_PREFIX"     [] { "INF" }
export def "log DEBUG_LEVEL_PREFIX"    [] { "DBG" }   

def parse-string-level [level: string] {
    (
        match $level {
            (log CRITICAL_LEVEL_PREFIX) => { log CRITICAL_LEVEL },
            "CRIT" => { log CRITICAL_LEVEL },
            "CRITICAL" => { log CRITICAL_LEVEL },
            (log ERROR_LEVEL_PREFIX) => { log ERROR_LEVEL },
            "ERROR" => { log ERROR_LEVEL },
            (log WARNING_LEVEL_PREFIX) => { log WARNING_LEVEL },
            "WARN" => { log WARNING_LEVEL },
            "WARNING" => { log WARNING_LEVEL },
            (log INFO_LEVEL_PREFIX) => { log INFO_LEVEL },
            "INFO" => { log INFO_LEVEL },
            (log DEBUG_LEVEL_PREFIX) => { log DEBUG_LEVEL },
            "DEBUG" => { log DEBUG_LEVEL },
            _ => { log INFO_LEVEL },
        }
    )
}

def parse-int-level [level: int] {
    (
        match $level {
            (log CRITICAL_LEVEL) => { log CRITICAL_LEVEL_PREFIX },
            (log ERROR_LEVEL) => { log ERROR_LEVEL_PREFIX },
            (log WARNING_LEVEL) => { log WARNING_LEVEL_PREFIX },
            (log INFO_LEVEL) => { log INFO_LEVEL_PREFIX },
            (log DEBUG_LEVEL) => { log DEBUG_LEVEL_PREFIX },
            _ => { log INFO_LEVEL_PREFIX },
        }
    )
}

def current-log-level [] {
    let env_level = ($env | get -i NU_LOG_LEVEL | default (log INFO_LEVEL))

    try {
        ($env_level | into int)
    } catch {
        parse-string-level $env_level
    }
}

def now [] {
    date now | date format "%Y-%m-%dT%H:%M:%S%.3f"
}

# Log critical message
export def "log critical" [message: string] {
    if (current-log-level) > (log CRITICAL_LEVEL) { return }

    print --stderr $"(ansi red_bold)CRT|(now)|($message)(ansi reset)"
}
# Log error message
export def "log error" [message: string] {
    if (current-log-level) > (log ERROR_LEVEL) { return }

    print --stderr $"(ansi red)ERR|(now)|($message)(ansi reset)"
}
# Log warning message
export def "log warning" [message: string] {
    if (current-log-level) > (log WARNING_LEVEL) { return }

    print --stderr $"(ansi yellow)WRN|(now)|($message)(ansi reset)"
}
# Log info message
export def "log info" [message: string] {
    if (current-log-level) > (log INFO_LEVEL) { return }

    print --stderr $"(ansi default)INF|(now)|($message)(ansi reset)"
}
# Log debug message
export def "log debug" [message: string] {
    if (current-log-level) > (log DEBUG_LEVEL) { return }

    print --stderr $"(ansi default_dimmed)DBG|(now)|($message)(ansi reset)"
}

# Log with custom message format and verbosity level
# 
# Format reference:
#   > %MSG% will be replaced by $message
#   > %DATE% will be replaced by the timestamp of log in standard Nushell's log format: "%Y-%m-%dT%H:%M:%S%.3f"
#   > %LEVEL% will be replaced by the standard Nushell's log verbosity prefixes, e.g. "CRT"
#
# Examples:
#   > std log custom "my message" $"(ansi yellow)[%LEVEL%]MY MESSAGE: %MSG% [%DATE%](ansi reset)" (std log WARNING_LEVEL) 
export def "log custom" [
    message: string, # Message inserted into the log template
    format: string, # A string to be printed into stderr. 
    log_level: int # Log's verbosity level
    ] {
    if (current-log-level) > ($log_level) { return }

    print --stderr ($format | str replace "%MSG%" $message | str replace "%DATE%" (now) | str replace "%LEVEL%" (parse-int-level $log_level | into string))
}