export def "log ANSI" [] {
    (
        {
            "CRITICAL": (ansi red_bold),
            "ERROR": (ansi red),
            "WARNING": (ansi yellow),
            "INFO": (ansi default),
            "DEBUG": (ansi default_dimmed)
        }
    )
}

export def "log LEVEL" [] {
    (
        {
            "CRITICAL": 50,
            "ERROR": 40,
            "WARNING": 30,
            "INFO": 20,
            "DEBUG": 10
        }
    )
}

export def "log PREFIX" [] {
    (
        {
            "CRITICAL": "CRT",
            "ERROR": "ERR",
            "WARNING": "WRN",
            "INFO": "INF",
            "DEBUG": "DBG"
        }
    )
}

export def "log SHORT_PREFIX" [] {
    (
        {
            "CRITICAL": "C",
            "ERROR": "E",
            "WARNING": "W",
            "INFO": "I",
            "DEBUG": "D"
        }
    )
}

def log-types [] {
    (
        {
            "CRITICAL": {
                "ansi": (log ANSI | get CRITICAL),
                "level": (log LEVEL | get CRITICAL),
                "prefix": (log PREFIX | get CRITICAL),
                "short_prefix": (log SHORT_PREFIX | get CRITICAL)
            },
            "ERROR": {
                "ansi": (log ANSI | get ERROR),
                "level": (log LEVEL | get ERROR),
                "prefix": (log PREFIX | get ERROR),
                "short_prefix": (log SHORT_PREFIX | get ERROR)
            },
            "WARNING": {
                "ansi": (log ANSI | get WARNING),
                "level": (log LEVEL | get WARNING),
                "prefix": (log PREFIX | get WARNING),
                "short_prefix": (log SHORT_PREFIX | get WARNING)
            }, 
            "INFO": {
                "ansi": (log ANSI | get INFO),
                "level": (log LEVEL | get INFO),
                "prefix": (log PREFIX | get INFO),
                "short_prefix": (log SHORT_PREFIX | get INFO)
            }, 
            "DEBUG": {
                "ansi": (log ANSI | get DEBUG),
                "level": (log LEVEL | get DEBUG),
                "prefix": (log PREFIX | get DEBUG),
                "short_prefix": (log SHORT_PREFIX | get DEBUG)
            },            
        }
    )
}


def parse-string-level [
    level: string
] {
    let prefixes = (log PREFIX)
    let short_prefixes = (log SHORT_PREFIX)
    let levels = (log LEVEL)

    if $level in [$prefixes.CRITICAL $short_prefixes.CRITICAL "CRIT" "CRITICAL"] {
        $levels.CRITICAL
    } else if $level in [$prefixes.ERROR $short_prefixes.ERROR "ERROR" ] {
        $levels.ERROR
    } else if $level in [$prefixes.WARNING $short_prefixes.WARNING "WARN" "WARNING"] {
        $levels.WARNING
    } else if $level in [$prefixes.DEBUG $short_prefixes.DEBUG "DEBUG"] {
        $levels.DEBUG
    } else {
        $levels.INFO
    }
}


def parse-int-level [
    level: int,
    --short (-s)
] {
    let prefixes = (log PREFIX)
    let short_prefixes = (log SHORT_PREFIX)
    let levels = (log LEVEL)

    if $level >= $levels.CRITICAL {
        if $short {
            $short_prefixes.CRITICAL
        } else {
            $prefixes.CRITICAL
        }
    } else if $level >= $levels.ERROR {
        if $short {
            $short_prefixes.ERROR
        } else {
            $prefixes.ERROR
        }
    } else if $level >= $levels.WARNING {
        if $short {
            $short_prefixes.WARNING
        } else {
            $prefixes.WARNING
        }
    } else if $level >= $levels.INFO {
        if $short {
            $short_prefixes.INFO
        } else {
            $prefixes.INFO
        }
    } else {
        if $short {
            $short_prefixes.DEBUG
        } else {
            $prefixes.DEBUG
        }
    }
}

def current-log-level [] {
    let env_level = ($env.NU_LOG_LEVEL? | default (log LEVEL | get INFO))

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
] {
    let prefix = if $short {
        $formatting.short_prefix
    } else {
        $formatting.prefix
    }

    print --stderr $"($formatting.ansi)(now)|($prefix)|(ansi u)($message)(ansi reset)"
}

# Log a critical message
export def "log critical" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    let formatting = (log-types | get CRITICAL)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short
}

# Log an error message
export def "log error" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    let formatting = (log-types | get ERROR)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short
}

# Log a warning message
export def "log warning" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    let formatting = (log-types | get WARNING)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short
}

# Log an info message
export def "log info" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    let formatting = (log-types | get INFO)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short
}

# Log a debug message
export def "log debug" [
    message: string, # A message
    --short (-s) # Whether to use a short prefix
] {
    let formatting = (log-types | get DEBUG)

    if (current-log-level) > ($formatting.level) {
        return
    }

    log-formatted $formatting $message $short
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
export def "log custom" [
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
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    })
}
