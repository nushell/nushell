def borrow-year [from: record, current: record] {
    mut current = $current

    $current.year = $current.year - 1
    $current.month = $current.month + 12

    $current
}

def leap-year-days [year] {
    if $year mod 400 == 0  {
        29
    } else if $year mod 4 == 0 and $year mod 100 != 0 {
        29
    } else {
        28
    }
}

def borrow-month [from: record, current: record] {
    mut current = $current
    if $from.month in [1, 3, 5, 7, 8, 10, 12] {
        $current.day = $current.day + 31
        $current.month = $current.month - 1
        if $current.month < 0 {
            $current = (borrow-year $from $current)
        }
    } else if $from.month in [4, 6, 9, 11] {
        $current.day = $current.day + 30
        $current.month = $current.month - 1
        if $current.month < 0 {
            $current = (borrow-year $from $current)
        }
    } else {
        # oh February
        let num_days_feb = (leap-year-days $current.year)
        $current.day = $current.day + $num_days_feb
        $current.month = $current.month - 1
        if $current.month < 0 {
            $current = (borrow-year $from $current)
        }
    }

    $current
}

def borrow-day [from: record, current: record] {
    mut current = $current
    $current.hour = $current.hour + 24
    $current.day = $current.day - 1
    if $current.day < 0 {
        $current = (borrow-month $from $current)
    }

    $current
}

def borrow-hour [from: record, current: record] {
    mut current = $current
    $current.minute = $current.minute + 60
    $current.hour = $current.hour - 1
    if $current.hour < 0 {
        $current = (borrow-day $from $current)
    }

    $current
}

def borrow-minute [from: record, current: record] {
    mut current = $current
    $current.second = $current.second + 60
    $current.minute = $current.minute - 1
    if $current.minute < 0 {
        $current = (borrow-hour $from $current)
    }

    $current
}

def borrow-second [from: record, current: record] {
    mut current = $current
    $current.nanosecond = $current.nanosecond + 1_000_000_000
    $current.second = $current.second - 1
    if $current.second < 0 {
        $current = (borrow-minute $from $current)
    }

    $current
}

# Subtract later from earlier datetime and return the unit differences as a record
# Example:
# > dt datetime-diff 2023-05-07T04:08:45+12:00 2019-05-10T09:59:12-07:00
# ╭─────────────┬────╮
# │ year        │ 3  │
# │ month       │ 11 │
# │ day         │ 26 │
# │ hour        │ 23 │
# │ minute      │ 9  │
# │ second      │ 33 │
# │ millisecond │ 0  │
# │ microsecond │ 0  │
# │ nanosecond  │ 0  │
# ╰─────────────┴────╯
export def datetime-diff [
        later: datetime, # a later datetime
        earlier: datetime  # earlier (starting) datetime
    ] {
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

    mut result = { year: ($from_expanded.year - $to_expanded.year), month: ($from_expanded.month - $to_expanded.month), day:0, hour:0, minute:0, second:0, millisecond:0, microsecond:0, nanosecond:0}

    if $result.month < 0 {
        $result = (borrow-year $from_expanded $result)
    }

    $result.day = $from_expanded.day - $to_expanded.day
    if $result.day < 0 {
        $result = (borrow-month $from_expanded $result)
    }

    $result.hour = $from_expanded.hour - $to_expanded.hour
    if $result.hour < 0 {
        $result = (borrow-day $from_expanded $result)
    }

    $result.minute = $from_expanded.minute - $to_expanded.minute
    if $result.minute < 0 {
        $result = (borrow-hour $from_expanded $result)
    }

    $result.second = $from_expanded.second - $to_expanded.second
    if $result.second < 0 {
        $result = (borrow-minute $from_expanded $result)
    }

    $result.nanosecond = $from_expanded.nanosecond - $to_expanded.nanosecond
    if $result.nanosecond < 0 {
        $result = (borrow-second $from_expanded $result)
    }

    $result.millisecond = ($result.nanosecond / 1_000_000 | into int) # don't want a float
    $result.microsecond = (($result.nanosecond mod 1_000_000) / 1_000 | into int)
    $result.nanosecond = ($result.nanosecond mod 1_000 | into int)

    $result
}

# Convert record from datetime-diff into humanized string
# Example:
# > dt pretty-print-duration (dt datetime-diff 2023-05-07T04:08:45+12:00 2019-05-10T09:59:12+12:00)
# 3yrs 11months 27days 18hrs 9mins 33secs
export def pretty-print-duration [dur: record] {
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
