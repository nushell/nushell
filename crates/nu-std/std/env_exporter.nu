export-env {
    # log.nu
    let-env LOG_ANSI = {
        "CRITICAL": (ansi red_bold),
        "ERROR": (ansi red),
        "WARNING": (ansi yellow),
        "INFO": (ansi default),
        "DEBUG": (ansi default_dimmed)
    }

    let-env LOG_LEVEL = {
        "CRITICAL": 50,
        "ERROR": 40,
        "WARNING": 30,
        "INFO": 20,
        "DEBUG": 10
    }

    let-env LOG_PREFIX = {
        "CRITICAL": "CRT",
        "ERROR": "ERR",
        "WARNING": "WRN",
        "INFO": "INF",
        "DEBUG": "DBG"
    }

    let-env LOG_SHORT_PREFIX = {
        "CRITICAL": "C",
        "ERROR": "E",
        "WARNING": "W",
        "INFO": "I",
        "DEBUG": "D"
    }

    let-env LOG_FORMAT = $"%ANSI_START%%DATE%|%LEVEL%|(ansi u)%MSG%%ANSI_STOP%"
}