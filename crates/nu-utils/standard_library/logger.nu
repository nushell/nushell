def CRITICAL_LEVEL [] { 50 }
def ERROR_LEVEL    [] { 40 }
def WARNING_LEVEL  [] { 30 }
def INFO_LEVEL     [] { 20 }
def DEBUG_LEVEL    [] { 10 }

def current-log-level [] {
    let env_level = try {
        $env.NU_LOG_LEVEL
    } catch {
        (INFO_LEVEL)
    }
    try {
        return ($env_level | into int)
    } catch {
        (
            if $env_level == "CRITICAL" { return (CRITICAL_LEVEL)}
            else if $env_level == "CRIT" { return (CRITICAL_LEVEL)}
            else if $env_level == "ERROR" { return (ERROR_LEVEL) }
            else if $env_level == "WARNING" { return (WARNING_LEVEL) }
            else if $env_level == "WARN" { return (WARNING_LEVEL) }
            else if $env_level == "INFO" { return (INFO_LEVEL) }
            else if $env_level == "DEBUG" { return (DEBUG_LEVEL) }
            else { return (INFO_LEVEL) }
        )
    }
}

# Log critical message
export def "log critical" [message: string] {
    if (current-log-level) > (CRITICAL_LEVEL) { return }
    echo $"(ansi red_bold)CRIT  ($message)(ansi reset)"
}
# Log error message
export def "log error" [message: string] {
    if (current-log-level) > (ERROR_LEVEL) { return }
    echo $"(ansi red)ERROR ($message)(ansi reset)"
}
# Log warning message
export def "log warning" [message: string] {
    if (current-log-level) > (WARNING_LEVEL) { return }
    echo $"(ansi yellow)WARN  ($message)(ansi reset)"
}
# Log info message
export def "log info" [message: string] {
    if (current-log-level) > (INFO_LEVEL) { return }
    echo $"(ansi white)INFO  ($message)(ansi reset)"
}
# Log debug message
export def "log debug" [message: string] {
    if (current-log-level) > (DEBUG_LEVEL) { return }
    echo $"(ansi grey)DEBUG ($message)(ansi reset)"
}
