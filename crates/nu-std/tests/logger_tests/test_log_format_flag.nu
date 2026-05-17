use std/testing *
use std *
use std/log *
use std/assert
use commons.nu *

def run-command [
    system_level,
    message_level,
    message,
    --format: string,
    --short
] {
    if $short {
        ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log ($message_level) --format "($format)" --short "($message)"'
    } else {
        ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log ($message_level) --format "($format)" "($message)"'
    }
    | complete | get --optional stderr
}


def "assert formatted" [
    message: string,
    format: string,
    command_level: string
    --short
] {
    let output = (run-command "debug" $command_level $message --format $format)
    let prefix = if $short {
            (log-short-prefix | get ($command_level | str upcase))
        } else {
            (log-prefix | get ($command_level | str upcase))
        }
    let ansi = if $short {
            (log-ansi | get ($command_level | str upcase))
        } else {
            (log-ansi | get ($command_level | str upcase))
        }

    assert equal ($output | str trim --right) (format-message $message $format $prefix $ansi)
}

@test
def format_flag [] {
    assert formatted "test" "25 %MSG% %ANSI_START% %LEVEL%%ANSI_STOP%" critical
    assert formatted "test" "25 %MSG% %ANSI_START% %LEVEL%%ANSI_STOP%" error
    assert formatted "test" "25 %MSG% %ANSI_START% %LEVEL%%ANSI_STOP%" warning
    assert formatted "test" "25 %MSG% %ANSI_START% %LEVEL%%ANSI_STOP%" info
    assert formatted "test" "25 %MSG% %ANSI_START% %LEVEL%%ANSI_STOP%" debug
    assert formatted --short "test" "TEST %ANSI_START% %MSG%%ANSI_STOP%" critical
    assert formatted --short "test" "TEST %ANSI_START% %MSG%%ANSI_STOP%" error
    assert formatted --short "test" "TEST %ANSI_START% %MSG%%ANSI_STOP%" warning
    assert formatted --short "test" "TEST %ANSI_START% %MSG%%ANSI_STOP%" info
    assert formatted --short "test" "TEST %ANSI_START% %MSG%%ANSI_STOP%" debug
}
