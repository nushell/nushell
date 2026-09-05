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
    --context :record
    --short
] {
    mut args = []

    if $short {
      $args = $args | append ["--short"]
    }

    if $context != null {
      $args = $args | append ["--context" ($context | to nuon)]
    }

    let args = $args | str join ' '

    ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log ($message_level) ($args) "($message)"'
    | complete | get --optional stderr
}


def "assert formatted" [
    message: string,
    format: string,
    command_level: string
    --context :record
    --short
] {

    let prefix = if $short {
            (log-short-prefix | get ($command_level | str uppercase))
        } else {
            (log-prefix | get ($command_level | str uppercase))
        }
    let ansi = if $short {
            (log-ansi | get ($command_level | str uppercase))
        } else {
            (log-ansi | get ($command_level | str uppercase))
        }

    let output = if ($context | is-empty) {
      (run-command "debug" $command_level $message --format $format)
    } else {
      (run-command "debug" $command_level $message --context $context --format $format)
    }

    if $context == null {
      assert equal ($output | str trim --right) (format-message $message $format $prefix $ansi)
    } else {
      assert equal ($output | str trim --right) (format-message --context $context $message $format $prefix $ansi)
    }
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
    assert formatted "test" --context {var: value} "TEST %ANSI_START% %MSG%%ANSI_STOP%" critical
    assert formatted "test" --context {var: value} "TEST %ANSI_START% %MSG%%ANSI_STOP%" error
    assert formatted "test" --context {var: value} "TEST %ANSI_START% %MSG%%ANSI_STOP%" warning
    assert formatted "test" --context {var: value} "TEST %ANSI_START% %MSG%%ANSI_STOP%" info
    assert formatted "test" --context {var: value} "TEST %ANSI_START% %MSG%%ANSI_STOP%" debug
}
