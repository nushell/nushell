export def log-ansi [] {
    {
        "CRITICAL": (ansi red_bold),
        "ERROR": (ansi red),
        "WARNING": (ansi yellow),
        "INFO": (ansi default),
        "DEBUG": (ansi default_dimmed)
    }
}

export def log-level [] {
    {
        "CRITICAL": 50,
        "ERROR": 40,
        "WARNING": 30,
        "INFO": 20,
        "DEBUG": 10
    }
}
export def log-prefix [] {
    {
        "CRITICAL": "CRT",
        "ERROR": "ERR",
        "WARNING": "WRN",
        "INFO": "INF",
        "DEBUG": "DBG"
    }
}
export def log-short-prefix [] {
    {
        "CRITICAL": "C",
        "ERROR": "E",
        "WARNING": "W",
        "INFO": "I",
        "DEBUG": "D"
    }
}
export-env {
    $env.NU_LOG_FORMAT = $env.NU_LOG_FORMAT? | default "%ANSI_START%%DATE%|%LEVEL%|%MSG%%ANSI_STOP%"
    $env.NU_LOG_DATE_FORMAT = $env.NU_LOG_DATE_FORMAT? | default "%Y-%m-%dT%H:%M:%S%.3f"
}

def log-types [] {
    (
        {
            "CRITICAL": {
                "ansi": (log-ansi).CRITICAL,
                "level": (log-level).CRITICAL,
                "prefix": (log-prefix).CRITICAL,
                "short_prefix": (log-short-prefix).CRITICAL
            },
            "ERROR": {
                "ansi": (log-ansi).ERROR,
                "level": (log-level).ERROR,
                "prefix": (log-prefix).ERROR,
                "short_prefix": (log-short-prefix).ERROR
            },
            "WARNING": {
                "ansi": (log-ansi).WARNING,
                "level": (log-level).WARNING,
                "prefix": (log-prefix).WARNING,
                "short_prefix": (log-short-prefix).WARNING
            },
            "INFO": {
                "ansi": (log-ansi).INFO,
                "level": (log-level).INFO,
                "prefix": (log-prefix).INFO,
                "short_prefix": (log-short-prefix).INFO
            },
            "DEBUG": {
                "ansi": (log-ansi).DEBUG,
                "level": (log-level).DEBUG,
                "prefix": (log-prefix).DEBUG,
                "short_prefix": (log-short-prefix).DEBUG
            }
        }
    )
}

def parse-string-level [
    level: string
] {
    let level = ($level | str upcase)

    if $level in [(log-prefix).CRITICAL (log-short-prefix).CRITICAL "CRIT" "CRITICAL"] {
        (log-level).CRITICAL
    } else if $level in [(log-prefix).ERROR (log-short-prefix).ERROR "ERROR"] {
        (log-level).ERROR
    } else if $level in [(log-prefix).WARNING (log-short-prefix).WARNING "WARN" "WARNING"] {
        (log-level).WARNING
    } else if $level in [(log-prefix).DEBUG (log-short-prefix).DEBUG "DEBUG"] {
        (log-level).DEBUG
    } else {
        (log-level).INFO
    }
}

def parse-int-level [
    level: int,
    --short (-s)
] {
    if $level >= (log-level).CRITICAL {
        if $short {
            (log-short-prefix).CRITICAL
        } else {
            (log-prefix).CRITICAL
        }
    } else if $level >= (log-level).ERROR {
        if $short {
            (log-short-prefix).ERROR
        } else {
            (log-prefix).ERROR
        }
    } else if $level >= (log-level).WARNING {
        if $short {
            (log-short-prefix).WARNING
        } else {
            (log-prefix).WARNING
        }
    } else if $level >= (log-level).INFO {
        if $short {
            (log-short-prefix).INFO
        } else {
            (log-prefix).INFO
        }
    } else {
        if $short {
            (log-short-prefix).DEBUG
        } else {
            (log-prefix).DEBUG
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default (log-level).INFO)

    try {
        $env_level | into int
    } catch {
        parse-string-level $env_level
    }
}

def now [] {
    date now | format date $env.NU_LOG_DATE_FORMAT
}

def handle-log [
    message: string,
    formatting: record,
    format_string: string,
    short: bool
] {
    let log_format = if ($format_string | is-empty) {
        $env.NU_LOG_FORMAT
    } else {
        $format_string
    }

    let prefix = if $short {
        $formatting.short_prefix
    } else {
        $formatting.prefix
    }

    custom $message $log_format $formatting.level --level-prefix $prefix --ansi $formatting.ansi
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
    let format = $format | default ""
    handle-log $message (log-types | get CRITICAL)  $format $short
}

# Log an error message
export def error [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message (log-types | get ERROR) $format $short
}

# Log a warning message
export def warning [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message (log-types | get WARNING) $format $short
}

# Log an info message
export def info [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message (log-types | get INFO) $format $short
}

# Log a debug message
export def debug [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message (log-types | get DEBUG) $format $short
}

def log-level-deduction-error [
    type: string
    span: record<start: int, end: int>
    log_level: int
] {
    error make {
        msg: $"(ansi red_bold)Cannot deduce ($type) for given log level: ($log_level).(ansi reset)"
        label: {
            text: ([
                 "Invalid log level."
                $"        Available log levels in log-level:"
                 (log-level | to text | lines | each {|it| $"            ($it)" } | to text)
            ] | str join "\n")
            span: $span
        }
    }
}

# Log a message with a specific format and verbosity level, with either configurable or auto-deduced %LEVEL% and %ANSI_START% placeholder extensions
export def custom [
    message: string, # A message
    format: string, # A format (for further reference: help std log)
    log_level: int # A log level (has to be one of the log-level values for correct ansi/prefix deduction)
    --level-prefix (-p): string # %LEVEL% placeholder extension
    --ansi (-a): string # %ANSI_START% placeholder extension
] {
    if (current-log-level) > ($log_level) {
        return
    }

    let valid_levels_for_defaulting = [
        (log-level).CRITICAL
        (log-level).ERROR
        (log-level).WARNING
        (log-level).INFO
        (log-level).DEBUG
    ]

    let prefix = if ($level_prefix | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            log-level-deduction-error "log level prefix" (metadata $log_level).span $log_level
        }

        parse-int-level $log_level

    } else {
        $level_prefix
    }

    let use_color = ($env | get config? | get use_ansi_coloring? | $in != false)
    let ansi = if not $use_color {
        ""
    } else if ($ansi | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            log-level-deduction-error "ansi" (metadata $log_level).span $log_level
        }

        (
            log-types
            | values
            | each {|record|
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

def "nu-complete log-level" [] {
    log-level | transpose description value
}

# Change logging level
export def --env set-level [level: int@"nu-complete log-level"] {
    # Keep it as a string so it can be passed to child processes
    $env.NU_LOG_LEVEL = $level | into string
}
