use std *
use commons.nu *

def run-command [
    system_level: string,
    message: string,
    format: string,
    log_level: int,
    --level-prefix: string,
    --ansi: string
] {
    if ($level_prefix | is-empty) {
        if ($ansi | is-empty) {
            ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level)'
        } else {
            ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level) --ansi "($ansi)"'
        }
    } else {
        ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level) --level-prefix "($level_prefix)" --ansi "($ansi)"'
    }
    | complete | get --ignore-errors stderr
}

#[test]
def errors_during_deduction [] {
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25) "Cannot deduce log level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --ansi (ansi red)) "Cannot deduce log level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --level-prefix "abc") "Cannot deduce ansi for given log level"
}

#[test]
def valid_calls [] {
    assert equal (run-command "DEBUG" "msg" "%MSG%" 25 --level-prefix "abc" --ansi (ansi default) | str trim --right) "msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) $"($env.LOG_PREFIX.INFO) msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" --level-prefix "abc" 20 | str trim --right) "abc msg"
    assert equal (run-command "INFO" "msg" "%ANSI_START%%LEVEL% %MSG%%ANSI_STOP%" $env.LOG_LEVEL.CRITICAL | str trim --right) $"($env.LOG_ANSI.CRITICAL)CRT msg(ansi reset)"
}

#[test]
def log_level_handling [] {
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) $"($env.LOG_PREFIX.INFO) msg"
    assert equal (run-command "WARNING" "msg" "%LEVEL% %MSG%" 20 | str trim --right) ""
}
