use std *
use commons.nu *

def run [ 
    message: string,
    level: string,
    --short
] {
    let config_filename = "crates/nu-std/tests/logger_tests/logger_env_test_config.nu"

    do {
        if $short {
            ^$nu.current-exe --commands $'use std; use ($config_filename); NU_LOG_LEVEL=($level) std log ($level) --short ($message)'
        } else {
            ^$nu.current-exe --commands $'use std; use ($config_filename); NU_LOG_LEVEL=($level) std log ($level) ($message)'
        }
    } | complete | get --ignore-errors stderr
}

export def "test_logger_uses_env" [] {   
    let ansi = {
        "CRITICAL": (ansi green),
        "ERROR": (ansi blue),
        "WARNING": (ansi green_bold),
        "INFO": (ansi blue_bold),
        "DEBUG": (ansi red)
    }

    let level = {
        "CRITICAL": 5,
        "ERROR": 4,
        "WARNING": 3,
        "INFO": 2,
        "DEBUG": 1
    }

    let prefix = {
        "CRITICAL": "CT",
        "ERROR": "ER",
        "WARNING": "WN",
        "INFO": "IF",
        "DEBUG": "DG"
    }

    let short_prefix = {
        "CRITICAL": "CR",
        "ERROR": "ER",
        "WARNING": "WA",
        "INFO": "IN",
        "DEBUG": "DE"
    }

    let format = $"%ANSI_START% | %LEVEL% | %MSG%%ANSI_STOP%"

    let message = "abc"

    assert equal (run $message "debug" | str trim --right) (format-message $message $format $prefix.DEBUG $ansi.DEBUG)
    assert equal (run $message "info" | str trim --right) (format-message $message $format $prefix.INFO $ansi.INFO)
    assert equal (run $message "warning" | str trim --right) (format-message $message $format $prefix.WARNING $ansi.WARNING)
    assert equal (run $message "error" | str trim --right) (format-message $message $format $prefix.ERROR $ansi.ERROR)
    assert equal (run $message "critical" | str trim --right) (format-message $message $format $prefix.CRITICAL $ansi.CRITICAL)
}