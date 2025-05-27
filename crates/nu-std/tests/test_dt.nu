use std/testing *
use std/assert
use std/dt *

@test
def equal_times [] {
    let t1 = (date now)
    assert equal (datetime-diff $t1 $t1) ({year:0, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def one_ns_later [] {
    let t1 = (date now)
    assert equal (datetime-diff ($t1 + 1ns) $t1) ({year:0, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:1})
}

@test
def one_yr_later [] {
    let t1 = ('2022-10-1T0:1:2z' | into datetime)   # a date for which one year later is 365 days, since duration doesn't support year or month
    assert equal (datetime-diff ($t1 + 365day) $t1) ({year:1, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def carry_ripples [] {
    let t1 = ('2023-10-9T0:0:0z' | into datetime)
    let t2 = ('2022-10-9T0:0:0.000000001z' | into datetime)
    assert equal (datetime-diff $t1 $t2) ({year:0, month:11, day:30, hour:23, minute:59, second:59, millisecond:999, microsecond:999 nanosecond:999})
}

@test
def earlier_arg_must_be_less_or_equal_later [] {
    let t1 = ('2022-10-9T0:0:0.000000001z' | into datetime)
    let t2 = ('2023-10-9T0:0:0z' | into datetime)
    assert error {|| (datetime-diff $t1 $t2)} 
}

@test
def banner_math_with_ms_us_ns [] {
    let t1 = 2023-05-07T04:08:45.582926123+12:00
    let t2 = 2019-05-10T09:59:12.967486456-07:00
    assert equal (datetime-diff $t1 $t2) ({year:3, month:11, day:26, hour:23, minute:9, second:32, millisecond:615, microsecond:439 nanosecond:667})
}

@test
def pp_skips_zeros [] {
    assert equal (pretty-print-duration {year:1, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0}) "1yr "
}

@test
def pp_doesnt_skip_neg [] { # datetime-diff can't return negative units, but prettyprint shouldn't skip them (if passed handcrafted record)
    assert equal (pretty-print-duration {year:-1, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0}) "-1yr "
}
