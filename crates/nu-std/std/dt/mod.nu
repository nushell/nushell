def leap-year-days [year] {
    if $year mod 400 == 0  {
        29
    } else if $year mod 4 == 0 and $year mod 100 != 0 {
        29
    } else {
        28
    }
}

def days-in-month [month: int, year: int] {
    if $month in [1, 3, 5, 7, 8, 10, 12] {
        31
    } else if $month in [4, 6, 9, 11] {
        30
    } else {
        (leap-year-days $year)
    }
}

# Add n months to a civil date, clamping the day to the resulting month's length.
def add-months-clamp [year: int, month: int, day: int, n: int] {
    let total = $month + $n
    let ny = $year + (($total - 1) // 12)
    let nm = (($total - 1) mod 12) + 1
    let dim = (days-in-month $nm $ny)
    let new_day = if $day > $dim { $dim } else { $day }
    { year: $ny, month: $nm, day: $new_day }
}

def borrow-day [current: record] {
    mut current = $current
    $current.hour = $current.hour + 24
    $current.day = $current.day - 1

    $current
}

def borrow-hour [current: record] {
    mut current = $current
    $current.minute = $current.minute + 60
    $current.hour = $current.hour - 1
    if $current.hour < 0 {
        $current = (borrow-day $current)
    }

    $current
}

def borrow-minute [current: record] {
    mut current = $current
    $current.second = $current.second + 60
    $current.minute = $current.minute - 1
    if $current.minute < 0 {
        $current = (borrow-hour $current)
    }

    $current
}

def borrow-second [current: record] {
    mut current = $current
    $current.millisecond = $current.millisecond + 1_000
    $current.second = $current.second - 1
    if $current.second < 0 {
        $current = (borrow-minute $current)
    }

    $current
}

def borrow-millisecond [current: record] {
    mut current = $current
    $current.microsecond = $current.microsecond + 1_000
    $current.millisecond = $current.millisecond - 1
    if $current.millisecond < 0 {
        $current = (borrow-second $current)
    }

    $current
}

def borrow-microsecond [current: record] {
    mut current = $current
    $current.nanosecond = $current.nanosecond + 1_000
    $current.microsecond = $current.microsecond - 1
    if $current.microsecond < 0 {
        $current = (borrow-millisecond $current)
    }

    $current
}

# Subtract later from earlier datetime and return the unit differences as a record
@example "Get the difference between two dates" {
    dt datetime-diff 2023-05-07T04:08:45.582926123+12:00 2019-05-10T09:59:12.967486456-07:00
} --result {
    year: 3,
    month: 11,
    day: 25,
    hour: 23,
    minute: 9,
    second: 32,
    millisecond: 615,
    microsecond: 439,
    nanosecond: 667,
}
export def datetime-diff [
    later: datetime, # a later datetime
    earlier: datetime  # earlier (starting) datetime
]: [nothing -> record] {
    if $earlier > $later {
        let start = (metadata $later).span.start
        let end = (metadata $earlier).span.end
        error make {
            msg: "Incompatible arguments",
            label: {
                span: {
                    start: $start
                    end: $end
                }
                text: $"First datetime must be >= second, but was actually ($later - $earlier) less than it."
            }
        }
    }
    let from_expanded = ($later | date to-timezone utc | into record)
    let to_expanded = ($earlier | date to-timezone utc | into record)

    mut result = { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0, millisecond: 0, microsecond: 0, nanosecond: 0 }

    let from_time_ns = (($from_expanded.hour * 3600 + $from_expanded.minute * 60 + $from_expanded.second) * 1_000_000_000) + ($from_expanded.millisecond * 1_000_000) + ($from_expanded.microsecond * 1_000) + $from_expanded.nanosecond
    let to_time_ns = (($to_expanded.hour * 3600 + $to_expanded.minute * 60 + $to_expanded.second) * 1_000_000_000) + ($to_expanded.millisecond * 1_000_000) + ($to_expanded.microsecond * 1_000) + $to_expanded.nanosecond

    # Count the whole months between the dates from the raw day of month (and, on
    # equal days, the time of day), then measure the remaining days from the
    # anchor: the earlier date advanced by those months, its day clamped to the
    # anchor month's length. Borrowing a month by table lookup instead
    # under-counted the days whenever the earlier date sat at the end of a month
    # (Jan 31 -> Mar 1), while the clamped anchor alone would round such a short
    # month up into a whole one.
    mut total_months = (($from_expanded.year - $to_expanded.year) * 12) + ($from_expanded.month - $to_expanded.month)
    if ($from_expanded.day < $to_expanded.day) or (($from_expanded.day == $to_expanded.day) and ($to_time_ns > $from_time_ns)) {
        $total_months = $total_months - 1
    }
    let anchor = (add-months-clamp $to_expanded.year $to_expanded.month $to_expanded.day $total_months)
    $result.year = $total_months // 12
    $result.month = $total_months mod 12
    # The anchor always lands in the later date's month or the month before it.
    $result.day = if $anchor.month == $from_expanded.month {
        $from_expanded.day - $anchor.day
    } else {
        (days-in-month $anchor.month $anchor.year) - $anchor.day + $from_expanded.day
    }

    $result.hour = $from_expanded.hour - $to_expanded.hour
    if $result.hour < 0 {
        $result = (borrow-day $result)
    }

    $result.minute = $from_expanded.minute - $to_expanded.minute
    if $result.minute < 0 {
        $result = (borrow-hour $result)
    }

    $result.second = $from_expanded.second - $to_expanded.second
    if $result.second < 0 {
        $result = (borrow-minute $result)
    }

    $result.millisecond = $from_expanded.millisecond - $to_expanded.millisecond
    if $result.millisecond < 0 {
        $result = (borrow-second $result)
    }

    $result.microsecond = $from_expanded.microsecond - $to_expanded.microsecond
    if $result.microsecond < 0 {
        $result = (borrow-millisecond $result)
    }

    $result.nanosecond = $from_expanded.nanosecond - $to_expanded.nanosecond
    if $result.nanosecond < 0 {
        $result = (borrow-microsecond $result)
    }

    $result
}

# Convert record from datetime-diff into humanized string
@example "Format the difference between two dates into a human readable string" {
    dt pretty-print-duration (dt datetime-diff 2023-05-07T04:08:45+12:00 2019-05-10T09:59:12+12:00)
} --result "3yrs 11months 26days 18hrs 9mins 33secs"
export def pretty-print-duration [dur: record]: [nothing -> string] {
    mut result = ""
    if $dur.year != 0 {
        if $dur.year > 1 {
            $result = $"($dur.year)yrs "
        } else {
            $result = $"($dur.year)yr "
        }
    }
    if $dur.month != 0 {
        if $dur.month > 1 {
            $result = $"($result)($dur.month)months "
        } else {
            $result = $"($result)($dur.month)month "
        }
    }
    if $dur.day != 0 {
        if $dur.day > 1 {
            $result = $"($result)($dur.day)days "
        } else {
            $result = $"($result)($dur.day)day "
        }
    }
    if $dur.hour != 0 {
        if $dur.hour > 1 {
            $result = $"($result)($dur.hour)hrs "
        } else {
            $result = $"($result)($dur.hour)hr "
        }
    }
    if $dur.minute != 0 {
        if $dur.minute > 1 {
            $result = $"($result)($dur.minute)mins "
        } else {
            $result = $"($result)($dur.minute)min "
        }
    }
    if $dur.second != 0 {
        if $dur.second > 1 {
            $result = $"($result)($dur.second)secs "
        } else {
            $result = $"($result)($dur.second)sec "
        }
    }
    if $dur.millisecond != 0 {
        if $dur.millisecond > 1 {
            $result = $"($result)($dur.millisecond)ms "
        } else {
            $result = $"($result)($dur.millisecond)ms "
        }
    }
    if $dur.microsecond != 0 {
        if $dur.microsecond > 1 {
            $result = $"($result)($dur.microsecond)µs "
        } else {
            $result = $"($result)($dur.microsecond)µs "
        }
    }
    if $dur.nanosecond != 0 {
        if $dur.nanosecond > 1 {
            $result = $"($result)($dur.nanosecond)ns "
        } else {
            $result = $"($result)($dur.nanosecond)ns "
        }
    }

    $result
}
