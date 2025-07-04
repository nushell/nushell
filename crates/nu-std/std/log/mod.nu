const LOG_ANSI = {
    "CRITICAL": (ansi red_bold),
    "ERROR": (ansi red),
    "WARNING": (ansi yellow),
    "INFO": (ansi default),
    "DEBUG": (ansi default_dimmed)
}

export def log-ansi [] {$LOG_ANSI}

const LOG_LEVEL = {
    "CRITICAL": 50,
    "ERROR": 40,
    "WARNING": 30,
    "INFO": 20,
    "DEBUG": 10
}

export def log-level [] {$LOG_LEVEL}

const LOG_PREFIX = {
    "CRITICAL": "CRT",
    "ERROR": "ERR",
    "WARNING": "WRN",
    "INFO": "INF",
    "DEBUG": "DBG"
}

export def log-prefix [] {$LOG_PREFIX}

const LOG_SHORT_PREFIX = {
    "CRITICAL": "C",
    "ERROR": "E",
    "WARNING": "W",
    "INFO": "I",
    "DEBUG": "D"
}

export def log-short-prefix [] {$LOG_SHORT_PREFIX}

const LOG_FORMATS = {
    log: "%ANSI_START%%DATE%|%LEVEL%|%MSG%%ANSI_STOP%"
    date: "%Y-%m-%dT%H:%M:%S%.3f"
}

export-env {
    $env.NU_LOG_FORMAT = $env.NU_LOG_FORMAT? | default $LOG_FORMATS.log
    $env.NU_LOG_DATE_FORMAT = $env.NU_LOG_DATE_FORMAT? | default $LOG_FORMATS.date
}

const LOG_TYPES = {
    "CRITICAL": {
        "ansi": $LOG_ANSI.CRITICAL,
        "level": $LOG_LEVEL.CRITICAL,
        "prefix": $LOG_PREFIX.CRITICAL,
        "short_prefix": $LOG_SHORT_PREFIX.CRITICAL
    },
    "ERROR": {
        "ansi": $LOG_ANSI.ERROR,
        "level": $LOG_LEVEL.ERROR,
        "prefix": $LOG_PREFIX.ERROR,
        "short_prefix": $LOG_SHORT_PREFIX.ERROR
    },
    "WARNING": {
        "ansi": $LOG_ANSI.WARNING,
        "level": $LOG_LEVEL.WARNING,
        "prefix": $LOG_PREFIX.WARNING,
        "short_prefix": $LOG_SHORT_PREFIX.WARNING
    },
    "INFO": {
        "ansi": $LOG_ANSI.INFO,
        "level": $LOG_LEVEL.INFO,
        "prefix": $LOG_PREFIX.INFO,
        "short_prefix": $LOG_SHORT_PREFIX.INFO
    },
    "DEBUG": {
        "ansi": $LOG_ANSI.DEBUG,
        "level": $LOG_LEVEL.DEBUG,
        "prefix": $LOG_PREFIX.DEBUG,
        "short_prefix": $LOG_SHORT_PREFIX.DEBUG
    }
}

def parse-string-level [
    level: string
] {
    let level = ($level | str upcase)

    if $level in [$LOG_PREFIX.CRITICAL $LOG_SHORT_PREFIX.CRITICAL "CRIT" "CRITICAL"] {
        $LOG_LEVEL.CRITICAL
    } else if $level in [$LOG_PREFIX.ERROR $LOG_SHORT_PREFIX.ERROR "ERROR"] {
        $LOG_LEVEL.ERROR
    } else if $level in [$LOG_PREFIX.WARNING $LOG_SHORT_PREFIX.WARNING "WARN" "WARNING"] {
        $LOG_LEVEL.WARNING
    } else if $level in [$LOG_PREFIX.DEBUG $LOG_SHORT_PREFIX.DEBUG "DEBUG"] {
        $LOG_LEVEL.DEBUG
    } else {
        $LOG_LEVEL.INFO
    }
}

def parse-int-level [
    level: int,
    --short (-s)
] {
    if $level >= $LOG_LEVEL.CRITICAL {
        if $short {
            $LOG_SHORT_PREFIX.CRITICAL
        } else {
            $LOG_PREFIX.CRITICAL
        }
    } else if $level >= $LOG_LEVEL.ERROR {
        if $short {
            $LOG_SHORT_PREFIX.ERROR
        } else {
            $LOG_PREFIX.ERROR
        }
    } else if $level >= $LOG_LEVEL.WARNING {
        if $short {
            $LOG_SHORT_PREFIX.WARNING
        } else {
            $LOG_PREFIX.WARNING
        }
    } else if $level >= $LOG_LEVEL.INFO {
        if $short {
            $LOG_SHORT_PREFIX.INFO
        } else {
            $LOG_PREFIX.INFO
        }
    } else {
        if $short {
            $LOG_SHORT_PREFIX.DEBUG
        } else {
            $LOG_PREFIX.DEBUG
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default $LOG_LEVEL.INFO)

    try {
        $env_level | into int
    } catch {
        parse-string-level $env_level
    }
}

def now [] {
    date now | format date ($env.NU_LOG_DATE_FORMAT? | default $LOG_FORMATS.date)
}

def handle-log [
    message: string,
    formatting: record,
    format_string: string,
    short: bool
] {
    let log_format = $format_string | default -e $env.NU_LOG_FORMAT? | default $LOG_FORMATS.log

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
    handle-log $message ($LOG_TYPES.CRITICAL)  $format $short
}

# Log an error message
export def error [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message ($LOG_TYPES.ERROR) $format $short
}

# Log a warning message
export def warning [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message ($LOG_TYPES.WARNING) $format $short
}

# Log an info message
export def info [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message ($LOG_TYPES.INFO) $format $short
}

# Log a debug message
export def debug [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
    --format (-f): string # A format (for further reference: help std log)
] {
    let format = $format | default ""
    handle-log $message ($LOG_TYPES.DEBUG) $format $short
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
                 ($LOG_LEVEL | to text | lines | each {|it| $"            ($it)" } | to text)
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
        $LOG_LEVEL.CRITICAL
        $LOG_LEVEL.ERROR
        $LOG_LEVEL.WARNING
        $LOG_LEVEL.INFO
        $LOG_LEVEL.DEBUG
    ]

    let prefix = if ($level_prefix | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            log-level-deduction-error "log level prefix" (metadata $log_level).span $log_level
        }

        parse-int-level $log_level

    } else {
        $level_prefix
    }

    let use_color = ($env.config?.use_ansi_coloring? | $in != false)
    let ansi = if not $use_color {
        ""
    } else if ($ansi | is-empty) {
        if ($log_level not-in $valid_levels_for_defaulting) {
            log-level-deduction-error "ansi" (metadata $log_level).span $log_level
        }

        (
            $LOG_TYPES
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

    print --stderr (
        $format
            | str replace --all "%MSG%" $message
            | str replace --all "%DATE%" (now)
            | str replace --all "%LEVEL%" $prefix
            | str replace --all "%ANSI_START%" $ansi
            | str replace --all "%ANSI_STOP%" (ansi reset)

    )
}

def "nu-complete log-level" [] {
    $LOG_LEVEL | transpose description value
}

# Change logging level
export def --env set-level [level: int@"nu-complete log-level"] {
    # Keep it as a string so it can be passed to child processes
    $env.NU_LOG_LEVEL = $level | into string
}
