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
def date_floor_before_epoch [] {
    (
        assert equal
        (1969-12-31T23:30:00+00:00 | date floor 1hr)
        (1969-12-31T23:00:00+00:00)
    )
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
def date_ceil_before_epoch [] {
    (
        assert equal
        (1969-12-31T23:30:00+00:00 | date ceil 1hr)
        (1970-01-01T00:00:00+00:00)
    )
}
