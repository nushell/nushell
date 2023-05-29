export-env {
    use env_exporter.nu
}

def log-types [] {
    (
        {
            "CRITICAL": {
                "ansi": $env.LOG_ANSI.CRITICAL,
                "level": $env.LOG_LEVEL.CRITICAL,
                "prefix": $env.LOG_PREFIX.CRITICAL,
                "short_prefix": $env.LOG_SHORT_PREFIX.CRITICAL
            },
            "ERROR": {
                "ansi": $env.LOG_ANSI.ERROR,
                "level": $env.LOG_LEVEL.ERROR,
                "prefix": $env.LOG_PREFIX.ERROR,
                "short_prefix": $env.LOG_SHORT_PREFIX.ERROR
            },
            "WARNING": {
                "ansi": $env.LOG_ANSI.WARNING,
                "level": $env.LOG_LEVEL.WARNING,
                "prefix": $env.LOG_PREFIX.WARNING,
                "short_prefix": $env.LOG_SHORT_PREFIX.WARNING
            }, 
            "INFO": {
                "ansi": $env.LOG_ANSI.INFO,
                "level": $env.LOG_LEVEL.INFO,
                "prefix": $env.LOG_PREFIX.INFO,
                "short_prefix": $env.LOG_SHORT_PREFIX.INFO
            }, 
            "DEBUG": {
                "ansi": $env.LOG_ANSI.DEBUG,
                "level": $env.LOG_LEVEL.DEBUG,
                "prefix": $env.LOG_PREFIX.DEBUG,
                "short_prefix": $env.LOG_SHORT_PREFIX.DEBUG
            }            
        }
    )
}


def parse-string-level [
    level: string
] {
    if $level in [$env.LOG_PREFIX.CRITICAL $env.LOG_SHORT_PREFIX.CRITICAL "CRIT" "CRITICAL"] {
        $env.LOG_LEVEL.CRITICAL
    } else if $level in [$env.LOG_PREFIX.ERROR $env.LOG_SHORT_PREFIX.ERROR "ERROR" ] {
        $env.LOG_LEVEL.ERROR
    } else if $level in [$env.LOG_PREFIX.WARNING $env.LOG_SHORT_PREFIX.WARNING "WARN" "WARNING"] {
        $env.LOG_LEVEL.WARNING
    } else if $level in [$env.LOG_PREFIX.DEBUG $env.LOG_SHORT_PREFIX.DEBUG "DEBUG"] {
        $env.LOG_LEVEL.DEBUG
    } else {
        $env.LOG_LEVEL.INFO
    }
}


def parse-int-level [
    level: int,
    --short (-s)
] {
    if $level >= $env.LOG_LEVEL.CRITICAL {
        if $short {
            $env.LOG_SHORT_PREFIX.CRITICAL
        } else {
            $env.LOG_PREFIX.CRITICAL
        }
    } else if $level >= $env.LOG_LEVEL.ERROR {
        if $short {
            $env.LOG_SHORT_PREFIX.ERROR
        } else {
            $env.LOG_PREFIX.ERROR
        }
    } else if $level >= $env.LOG_LEVEL.WARNING {
        if $short {
            $env.LOG_SHORT_PREFIX.WARNING
        } else {
            $env.LOG_PREFIX.WARNING
        }
    } else if $level >= $env.LOG_LEVEL.INFO {
        if $short {
            $env.LOG_SHORT_PREFIX.INFO
        } else {
            $env.LOG_PREFIX.INFO
        }
    } else {
        if $short {
            $env.LOG_SHORT_PREFIX.DEBUG
        } else {
            $env.LOG_PREFIX.DEBUG
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default ($env.LOG_LEVEL.INFO))

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
    formatting: record
    message: string
    short: bool
    format: string
] {
    let prefix = if $short {
        $formatting.short_prefix
    } else {
        $formatting.prefix
    }

    let log_format = if ($format | is-empty) {
        $env.LOG_FORMAT
    } else {
        $format
    }

    print --stderr ([
        ["%MSG%" $message]
        ["%DATE%" (now)]
        ["%LEVEL%" $prefix]
        ["%ANSI_START%" $formatting.ansi]
        ["%ANSI_STOP%" (ansi reset)]
    ] | reduce --fold $log_format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    })
}

# Log a critical message
export def critical [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format
] {
    let formatting = (log-types | get CRITICAL)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short $format    
}

# Log an error message
export def error [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format
] {
    let formatting = (log-types | get ERROR)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short $format
}

# Log a warning message
export def warning [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format
] {
    let formatting = (log-types | get WARNING)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short $format
}

# Log an info message
export def info [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format
] {
    let formatting = (log-types | get INFO)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short $format
}

# Log a debug message
export def debug [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string
] {
    let formatting = (log-types | get DEBUG)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short $format
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
        ["%ANSI_STOP%" (ansi reset)]
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    })
}