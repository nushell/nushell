export-env {
    let-env LOG_ANSI = {
        "CRITICAL": (ansi red_bold),
        "ERROR": (ansi red),
        "WARNING": (ansi yellow),
        "INFO": (ansi default),
        "DEBUG": (ansi default_dimmed)
    }

    let-env LOG_LEVEL = {
        "CRITICAL": 50,
        "ERROR": 40,
        "WARNING": 30,
        "INFO": 20,
        "DEBUG": 10
    }

    let-env LOG_PREFIX = {
        "CRITICAL": "CRT",
        "ERROR": "ERR",
        "WARNING": "WRN",
        "INFO": "INF",
        "DEBUG": "DBG"
    }

    let-env LOG_SHORT_PREFIX = {
        "CRITICAL": "C",
        "ERROR": "E",
        "WARNING": "W",
        "INFO": "I",
        "DEBUG": "D"
    }

    let-env LOG_FORMAT = $"%ANSI_START%%DATE%|%LEVEL%|(ansi u)%MSG%%ANSI_STOP%"
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
    let level = ($level | str upcase)

    if $level in [$env.LOG_PREFIX.CRITICAL $env.LOG_SHORT_PREFIX.CRITICAL "CRIT" "CRITICAL"] {
        $env.LOG_LEVEL.CRITICAL
    } else if $level in [$env.LOG_PREFIX.ERROR $env.LOG_SHORT_PREFIX.ERROR "ERROR"] {
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

def handle-log [
    message: string,
    formatting: record,
    format_string: string,
    short: bool
] {
    let log_format = if ($format_string | is-empty) {
        $env.LOG_FORMAT
    } else {
        $format_string
    }

    let prefix = if $short {
        $formatting.short_prefix
    } else {
        $formatting.prefix
    }

    custom $message $log_format $formatting.level --level_prefix $prefix --ansi $formatting.ansi
}

# Logging module
#
# Log formatting placeholders:
# - %MSG%: message to be logged
# - %DATE%: date of log
# - %LEVEL%: string prefix for the log level
# - %ANSI_START%: ansi formatting 
# - %ANSI_STOP%: literally (ansi reset)
#
# Note: All placeholders are optional, so "" is still a valid format
#
# Example: $"%ANSI_START%%DATE%|%LEVEL%|(ansi u)%MSG%%ANSI_STOP%"
export def main [] {}

# Log a critical message
export def critical [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let formatting = (log-types | get CRITICAL)    

    if (current-log-level) > ($formatting.level) {
        return
    }

    handle-log $message $formatting $format $short
}

# Log an error message
export def error [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let formatting = (log-types | get ERROR)

    if (current-log-level) > ($formatting.level) {
        return
    }

    handle-log $message $formatting $format $short
}

# Log a warning message
export def warning [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let formatting = (log-types | get WARNING)

    if (current-log-level) > ($formatting.level) {
        return
    }

    handle-log $message $formatting $format $short
}

# Log an info message
export def info [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let formatting = (log-types | get INFO)

    if (current-log-level) > ($formatting.level) {
        return
    }

    handle-log $message $formatting $format $short
}

# Log a debug message
export def debug [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let formatting = (log-types | get DEBUG)

    if (current-log-level) > ($formatting.level) {
        return
    }

    handle-log $message $formatting $format $short
}

# Log a message with a specific format and verbosity level, with either configurable or auto-deduced %LEVEL% and %ANSI_START% placeholder extenions
export def custom [
    message: string, # A message
    format: string, # A format (for further reference: help std log)
    log_level: int # A log level (has to be one of the $env.LOG_LEVEL values for correct ansi/prefix deduction)
    --level_prefix (-p): string # %LEVEL% placeholder extension
    --ansi (-a): string # %ANSI_START% placeholder extension
] {
    if (current-log-level) > ($log_level) {
        return
    }

    let valid_levels_for_defaulting = [
        $env.LOG_LEVEL.CRITICAL
        $env.LOG_LEVEL.ERROR
        $env.LOG_LEVEL.WARNING
        $env.LOG_LEVEL.INFO
        $env.LOG_LEVEL.DEBUG
    ]

    let prefix = if ($level_prefix | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            let span = (metadata $log_level).span
            error make {
                "msg": $"Cannot deduce level prefix for given log level: ($log_level)"
                label: {
                    text: "Invalid log level for prefix auto-deduction"
                    start: $span.start
                    end: $span.end
                }
            }
        }
        
        parse-int-level $log_level

    } else {
        $level_prefix
    }

    let ansi = if ($ansi | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            let span = (metadata $log_level).span
            error make {
                "msg": $"Cannot deduce ansi for given log level: ($log_level)"
                label: {
                    text: "Invalid log level for ansi auto-deduction"
                    start: $span.start
                    end: $span.end
                }
            }
        }

        (
            log-types 
            |values
            |each {|record|
                if ($record.level == $log_level) {
                    $record.ansi
                }
            } | first
        ) 
    } else {
        $ansi
    }

    print --stderr ([
        ["%MSG%" $message]
        ["%DATE%" (now)]
        ["%LEVEL%" $prefix]
        ["%ANSI_START%" $ansi]
        ["%ANSI_STOP%" (ansi reset)]
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    })
}