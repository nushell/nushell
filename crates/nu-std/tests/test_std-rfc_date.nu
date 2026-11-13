use std/assert
use std/testing *
use std-rfc/date *

@test
def date_floor [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:00 | date floor 1hr)
        (2026-07-15T12:00:00-04:00)
    )
}

@test
def date_floor_day_duration [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:00 | date floor 1day)
        (2026-07-15T00:00:00-04:00)
    )
}

@test
def date_floor_fractional_timezone [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:30 | date floor 1day)
        (2026-07-15T00:00:00-04:30)
    )
}

@test
def date_floor_before_epoch [] {
    (
        assert equal
        (1969-12-31T23:30:00+00:00 | date floor 1hr)
        (1969-12-31T23:00:00+00:00)
    )
}

@test
def date_floor_timezone_with_seconds [] {
    let date = 1960-06-15T12:34:56Z | date to-timezone Africa/Monrovia
    let rounded = $date | date floor 1hr
    assert equal ($rounded | format date '%H:%M:%S %::z') '11:00:00 -00:44:30'
    assert equal ($rounded | date to-timezone UTC) 1960-06-15T11:44:30Z
}

@test
def date_ceil [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:00 | date ceil 1hr)
        (2026-07-15T13:00:00-04:00)
    )
}


@test
def date_ceil_day_duration [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:00 | date ceil 1day)
        (2026-07-16T00:00:00-04:00)
    )
}

@test
def date_ceil_fractional_timezone [] {
    (
        assert equal
        (2026-07-15T12:11:10-04:30 | date ceil 1day)
        (2026-07-16T00:00:00-04:30)
    )
}

@test
def date_ceil_before_epoch [] {
    (
        assert equal
        (1969-12-31T23:30:00+00:00 | date ceil 1hr)
        (1970-01-01T00:00:00+00:00)
    )
}

@test
def date_ceil_timezone_with_seconds [] {
    let date = 1960-06-15T12:34:56Z | date to-timezone Africa/Monrovia
    let rounded = $date | date ceil 1hr
    assert equal ($rounded | format date '%H:%M:%S %::z') '12:00:00 -00:44:30'
    assert equal ($rounded | date to-timezone UTC) 1960-06-15T12:44:30Z
}
