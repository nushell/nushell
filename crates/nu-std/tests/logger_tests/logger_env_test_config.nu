 export-env {
    let-env LOG_ANSI = {
        "CRITICAL": (ansi green),
        "ERROR": (ansi blue),
        "WARNING": (ansi green_bold),
        "INFO": (ansi blue_bold),
        "DEBUG": (ansi red)
    }

    let-env LOG_LEVEL = {
        "CRITICAL": 5,
        "ERROR": 4,
        "WARNING": 3,
        "INFO": 2,
        "DEBUG": 1
    }

    let-env LOG_PREFIX = {
        "CRITICAL": "CT",
        "ERROR": "ER",
        "WARNING": "WN",
        "INFO": "IF",
        "DEBUG": "DG"
    }

    let-env LOG_SHORT_PREFIX = {
        "CRITICAL": "CR",
        "ERROR": "ER",
        "WARNING": "WA",
        "INFO": "IN",
        "DEBUG": "DE"
    }

    let-env LOG_FORMAT = $"%ANSI_START% | %LEVEL% | %MSG%%ANSI_STOP%"
}
