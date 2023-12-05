use std *

#[test]
def env_log_ansi [] {
    assert equal $env.LOG_ANSI.CRITICAL (ansi red_bold)
    assert equal $env.LOG_ANSI.ERROR (ansi red)
    assert equal $env.LOG_ANSI.WARNING (ansi yellow)
    assert equal $env.LOG_ANSI.INFO (ansi default)
    assert equal $env.LOG_ANSI.DEBUG (ansi default_dimmed)
}

#[test]
def env_log_level [] {
    assert equal $env.LOG_LEVEL.CRITICAL 50
    assert equal $env.LOG_LEVEL.ERROR 40
    assert equal $env.LOG_LEVEL.WARNING 30
    assert equal $env.LOG_LEVEL.INFO 20
    assert equal $env.LOG_LEVEL.DEBUG 10
}

#[test]
def env_log_prefix [] {
    assert equal $env.LOG_PREFIX.CRITICAL "CRT"
    assert equal $env.LOG_PREFIX.ERROR "ERR"
    assert equal $env.LOG_PREFIX.WARNING "WRN"
    assert equal $env.LOG_PREFIX.INFO "INF"
    assert equal $env.LOG_PREFIX.DEBUG "DBG"
}

#[test]
def env_log_short_prefix [] {
    assert equal $env.LOG_SHORT_PREFIX.CRITICAL "C"
    assert equal $env.LOG_SHORT_PREFIX.ERROR "E"
    assert equal $env.LOG_SHORT_PREFIX.WARNING "W"
    assert equal $env.LOG_SHORT_PREFIX.INFO "I"
    assert equal $env.LOG_SHORT_PREFIX.DEBUG "D"
}

#[test]
def env_log_format [] {
    assert equal $env.NU_LOG_FORMAT $"%ANSI_START%%DATE%|%LEVEL%|%MSG%%ANSI_STOP%"
}
