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
    assert equal (datetime-diff $t1 $t2) ({year:0, month:11, day:29, hour:23, minute:59, second:59, millisecond:999, microsecond:999 nanosecond:999})
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
    assert equal (datetime-diff $t1 $t2) ({year:3, month:11, day:25, hour:23, minute:9, second:32, millisecond:615, microsecond:439 nanosecond:667})
}

@test
def borrow_month_uses_crossed_month [] {
    # Borrowing a month must add the days of the month that is actually crossed
    # (the month before the later date), not the later date's own month.
    # Jan 15 -> Mar 1 is 45 days = 1 month (Jan 15 -> Feb 15, 31 days) + 14 days.
    let t1 = ('2021-03-01T00:00:00z' | into datetime)
    let t2 = ('2021-01-15T00:00:00z' | into datetime)
    assert equal (datetime-diff $t1 $t2) ({year:0, month:1, day:14, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def borrow_february_uses_real_leap_year [] {
    # When the crossed month is February, its length depends on the real
    # calendar year of that February (2020 is a leap year -> 29 days).
    let t1 = ('2020-03-01T00:00:00z' | into datetime)
    let t2 = ('2020-01-15T00:00:00z' | into datetime)
    assert equal (datetime-diff $t1 $t2) ({year:0, month:1, day:15, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def pp_skips_zeros [] {
    assert equal (pretty-print-duration {year:1, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0}) "1yr "
}

@test
def pp_doesnt_skip_neg [] { # datetime-diff can't return negative units, but prettyprint shouldn't skip them (if passed handcrafted record)
    assert equal (pretty-print-duration {year:-1, month:0, day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0}) "-1yr "
}

@test
def month_end_to_first_of_month [] {
    # When the earlier date is at the end of a month (Jan 31) and the later
    # date is the 1st of a month two months later (Mar 1), the month borrow
    # must not under-count the days, leaving a negative or short day.
    let later = ('2023-03-01T00:00:00z' | into datetime)
    let earlier = ('2023-01-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:1, day:1, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def leap_month_end_to_first_of_month [] {
    # Same shape in a leap year: Jan 31 -> Mar 1 crosses a 29-day February.
    let later = ('2024-03-01T00:00:00z' | into datetime)
    let earlier = ('2024-01-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:1, day:1, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def month_end_30_to_first_of_month [] {
    # May 31 -> Jul 1 crosses June (30 days); the borrow must still land on day 1.
    let later = ('2023-07-01T00:00:00z' | into datetime)
    let earlier = ('2023-05-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:1, day:1, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def clamped_anchor_does_not_complete_a_month [] {
    # Jan 31 -> Feb 28 is not a completed month: the anchor's day is clamped to
    # Feb 28, but the raw day of month decides, so all 28 days remain as days.
    let later = ('2023-02-28T00:00:00z' | into datetime)
    let earlier = ('2023-01-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:0, day:28, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def month_end_to_last_day_of_30_day_month [] {
    # Same shape for a 30-day month: Mar 31 -> Apr 30 stays 30 days.
    let later = ('2023-04-30T00:00:00z' | into datetime)
    let earlier = ('2023-03-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:0, day:30, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def leap_day_to_clamped_anniversary [] {
    # Feb 29 -> Feb 28 a year later: the anchor is clamped, so the last day is
    # not yet a full year.
    let later = ('2025-02-28T00:00:00z' | into datetime)
    let earlier = ('2024-02-29T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:11, day:30, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def month_end_with_time_of_day [] {
    # A later time of day does not make up for the missing day of the month.
    let later = ('2023-02-28T13:00:00z' | into datetime)
    let earlier = ('2023-01-31T12:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:0, day:28, hour:1, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def month_end_borrowing_a_day_from_the_time_of_day [] {
    # The remaining 28 days still lose one to the time of day, leaving 27 days
    # 18 hours, and the borrow must not fall through to borrow a whole month.
    let later = ('2023-02-28T06:00:00z' | into datetime)
    let earlier = ('2023-01-31T12:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:0, day:27, hour:18, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def month_end_across_a_year_rollover [] {
    # Dec 31 -> Mar 1 keeps the year rollover and the clamped February anchor.
    let later = ('2023-03-01T00:00:00z' | into datetime)
    let earlier = ('2022-12-31T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:2, day:1, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}

@test
def clamped_day_that_is_not_a_month_end [] {
    # Jan 29 -> Mar 1: the anchor is Feb 28, i.e. the day was clamped even though
    # neither date is at a month end, and the extra day must still be counted.
    let later = ('2023-03-01T00:00:00z' | into datetime)
    let earlier = ('2023-01-29T00:00:00z' | into datetime)
    assert equal (datetime-diff $later $earlier) ({year:0, month:1, day:1, hour:0, minute:0, second:0, millisecond:0, microsecond:0 nanosecond:0})
}
