export def "log CRITICAL_LEVEL" [] { 50 }
export def "log ERROR_LEVEL"    [] { 40 }
export def "log WARNING_LEVEL"  [] { 30 }
export def "log INFO_LEVEL"     [] { 20 }
export def "log DEBUG_LEVEL"    [] { 10 }

def parse-string-level [level: string] {
    (
        if $level == "CRITICAL" { (log CRITICAL_LEVEL)}
        else if $level == "CRIT" { (log CRITICAL_LEVEL)}
        else if $level == "ERROR" { (log ERROR_LEVEL) }
        else if $level == "ERR" { (log ERROR_LEVEL) }
        else if $level == "WARNING" { (log WARNING_LEVEL) }
        else if $level == "WARN" { (log WARNING_LEVEL) }
        else if $level == "INFO" { (log INFO_LEVEL) }
        else if $level == "DEBUG" { (log DEBUG_LEVEL) }
        else { (log INFO_LEVEL) }
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
# # Usage:
# ```
# use std
# std log custom "my message" $"(ansi yellow)MY MESSAGE: %MSG%(ansi reset)" (std log WARNING_LEVEL)
# ```
export def "log custom" [
    message: string, # Message inserted into the log template
    format: string, # A string to be printed into stderr. Add %MSG% in place where the $message argument should be placed
    log_level: int # Log's verbosity level
    ] {
    if (current-log-level) > ($log_level) { return }

    print --stderr ($format | str replace "%MSG%" $message)
}