use std *
use std log *

#[test]
def env_log-ansi [] {
    assert equal (log-ansi).CRITICAL (ansi red_bold)
    assert equal (log-ansi).ERROR (ansi red)
    assert equal (log-ansi).WARNING (ansi yellow)
    assert equal (log-ansi).INFO (ansi default)
    assert equal (log-ansi).DEBUG (ansi default_dimmed)
}

#[test]
def env_log-level [] {
    assert equal (log-level).CRITICAL 50
    assert equal (log-level).ERROR 40
    assert equal (log-level).WARNING 30
    assert equal (log-level).INFO 20
    assert equal (log-level).DEBUG 10
}

#[test]
def env_log-prefix [] {
    assert equal (log-prefix).CRITICAL "CRT"
    assert equal (log-prefix).ERROR "ERR"
    assert equal (log-prefix).WARNING "WRN"
    assert equal (log-prefix).INFO "INF"
    assert equal (log-prefix).DEBUG "DBG"
}

#[test]
def env_log-short-prefix [] {
    assert equal (log-short-prefix).CRITICAL "C"
    assert equal (log-short-prefix).ERROR "E"
    assert equal (log-short-prefix).WARNING "W"
    assert equal (log-short-prefix).INFO "I"
    assert equal (log-short-prefix).DEBUG "D"
}

#[test]
def env_log_format [] {
    assert equal $env.NU_LOG_FORMAT $"%ANSI_START%%DATE%|%LEVEL%|%MSG%%ANSI_STOP%"
}
