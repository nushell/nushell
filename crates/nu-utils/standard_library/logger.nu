def CRITICAL_LEVEL [] { 50 }
def ERROR_LEVEL    [] { 40 }
def WARNING_LEVEL  [] { 30 }
def INFO_LEVEL     [] { 20 }
def DEBUG_LEVEL    [] { 10 }

def parse-string-level [level: string] {
    (
        if $level == "CRITICAL" { (CRITICAL_LEVEL)}
        else if $level == "CRIT" { (CRITICAL_LEVEL)}
        else if $level == "ERROR" { (ERROR_LEVEL) }
        else if $level == "WARNING" { (WARNING_LEVEL) }
        else if $level == "WARN" { (WARNING_LEVEL) }
        else if $level == "INFO" { (INFO_LEVEL) }
        else if $level == "DEBUG" { (DEBUG_LEVEL) }
        else { (INFO_LEVEL) }
    )
}

def current-log-level [] {
    let env_level = ($env | get -i NU_LOG_LEVEL | default (INFO_LEVEL))

    try {
        ($env_level | into int)
    } catch {
        parse-string-level $env_level
    }
}

# Log critical message
export def "log critical" [message: string] {
    if (current-log-level) > (CRITICAL_LEVEL) { return }

    print --stderr $"(ansi red_bold)CRIT  ($message)(ansi reset)"
}
# Log error message
export def "log error" [message: string] {
    if (current-log-level) > (ERROR_LEVEL) { return }

    print --stderr $"(ansi red)ERROR ($message)(ansi reset)"
}
# Log warning message
export def "log warning" [message: string] {
    if (current-log-level) > (WARNING_LEVEL) { return }

    print --stderr $"(ansi yellow)WARN  ($message)(ansi reset)"
}
# Log info message
export def "log info" [message: string] {
    if (current-log-level) > (INFO_LEVEL) { return }

    print --stderr $"(ansi white)INFO  ($message)(ansi reset)"
}
# Log debug message
export def "log debug" [message: string] {
    if (current-log-level) > (DEBUG_LEVEL) { return }

    print --stderr $"(ansi default_dimmed)DEBUG ($message)(ansi reset)"
}
