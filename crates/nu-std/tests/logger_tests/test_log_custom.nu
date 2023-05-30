use std *

def run-command [
    system_level: string,
    message: string,
    format: string,
    log_level: int,
    --level_prefix: string,
    --ansi: string
] {
    do {
        if ($level_prefix | is-empty) {
            if ($ansi | is-empty) {
                ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level)'
            } else {
                ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level) --ansi "($ansi)"'
            }
        } else {
            ^$nu.current-exe --commands $'use std; NU_LOG_LEVEL=($system_level) std log custom "($message)" "($format)" ($log_level) --level_prefix "($level_prefix)" --ansi "($ansi)"'
        }
    }  | complete | get --ignore-errors stderr
}

def now [] {
    date now | date format "%Y-%m-%dT%H:%M:%S%.3f"
}

def format-message [
    message: string,
    format: string
    prefix: string,
    ansi
] {
    [   
        ["%MSG%" $message]
        ["%DATE%" (now)]
        ["%LEVEL%" $prefix]
        ["%ANSI_START%" $ansi]
        ["%ANSI_STOP%" (ansi reset)]
    ] | reduce --fold $format {
        |it, acc| $acc | str replace --all $it.0 $it.1
    }
}

export def test_errors_during_deduction [] {
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25) "Cannot deduce level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --ansi (ansi red)) "Cannot deduce level prefix for given log level"
    assert str contains (run-command "DEBUG" "msg" "%MSG%" 25 --level_prefix "abc") "Cannot deduce ansi for given log level"
}

export def test_valid_calls [] {
    assert equal (run-command "DEBUG" "msg" "%MSG%" 25 --level_prefix "abc" --ansi (ansi default) | str trim --right) "msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) "INF msg"
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" --level_prefix "abc" 20 | str trim --right) "abc msg"
}

export def test_log_level_handling [] {
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) "INF msg"
    assert equal (run-command "WARNING" "msg" "%LEVEL% %MSG%" 20 | str trim --right) ""
}