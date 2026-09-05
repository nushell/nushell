use std/testing *
use std/assert
use commons.nu *

def run-command [
    system_level: string,
    message: string,
    format: string,
    log_level: int,
    --level-prefix: string,
    --context: record
    --ansi: string
] {
  mut args = []

  if ($level_prefix | is-not-empty) {
    $args = $args | append ["--level-prefix" $level_prefix]
  }

  if ($ansi | is-not-empty) {
    $args = $args | append ["--ansi" $ansi]
  }

  if ($context | is-not-empty) {
    $args = $args | append ["--context" ($context | to nuon)]
  }

  let args = $args | str join ' '

  ^$nu.current-exe --no-config-file --commands $'use std; use std/log; NU_LOG_LEVEL=($system_level) log custom ($args) "($message)" "($format)" ($log_level)'
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
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %CONTEXT%" --level-prefix "abc" 20 | str trim --right) 'abc var="value"'
    assert equal (run-command "INFO" "msg" "%ANSI_START%%LEVEL% %MSG% %CONTEXT%%ANSI_STOP%" ((log-level).CRITICAL) --context {var: value} | str trim --right) $'((log-ansi).CRITICAL)CRT msg var="value"(ansi reset)'
}

@test
def log-level_handling [] {
    use std/log *
    assert equal (run-command "DEBUG" "msg" "%LEVEL% %MSG%" 20 | str trim --right) $"((log-prefix).INFO) msg"
    assert equal (run-command "WARNING" "msg" "%LEVEL% %MSG%" 20 | str trim --right) ""
}
