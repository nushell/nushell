use std/testing *
use std/assert
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
            ^$nu.current-exe --no-config-file --commands $'use std/log; NU_LOG_LEVEL=($system_level) log custom "($message)" "($format)" ($log_level)'
        } else {
            ^$nu.current-exe --no-config-file --commands $'use std/log; NU_LOG_LEVEL=($system_level) log custom "($message)" "($format)" ($log_level) --ansi "($ansi)"'
        }
    } else {
        ^$nu.current-exe --no-config-file --commands $'use std/log; NU_LOG_LEVEL=($system_level) log custom "($message)" "($format)" ($log_level) --level-prefix "($level_prefix)" --ansi "($ansi)"'
    }
    | complete | get --optional stderr
}

@test
def errors_during_deduction [] {
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25) "Cannot deduce log level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --ansi (ansi red)) "Cannot deduce log level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --level-prefix "abc") "Cannot deduce ansi for given log level"
}

@test
def valid_calls [] {
    use std/log *
    assert equal (run-command "DEBUG" "msg" "%MSG%" 25 --level-prefix "abc" --ansi (ansi default) | str trim --right) "msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) $"((log-prefix).INFO) msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" --level-prefix "abc" 20 | str trim --right) "abc msg"
    assert equal (run-command "INFO" "msg" "%ANSI_START%%LEVEL% %MSG%%ANSI_STOP%" ((log-level).CRITICAL) | str trim --right) $"((log-ansi).CRITICAL)CRT msg(ansi reset)"
}

@test
def log-level_handling [] {
    use std/log *
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) $"((log-prefix).INFO) msg"
    assert equal (run-command "WARNING" "msg" "%LEVEL% %MSG%" 20 | str trim --right) ""
}
