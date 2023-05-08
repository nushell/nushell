def borrow-year [from: record, current: record] {
    mut current = $current

    $current.year = $current.year - 1
    $current.month = $current.month + 12

    $current
}

def leap-year-days [year] {
    # ($year mod 4) == 0 and (($year mod 100 != 0) or ($year mod 400 == 0))
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
        # let num_days_feb = if $current.from mod 400 == 0 { 29 } else if $current.from mod 4 == 0 and $current.from mod 100 != 0 { 29 } else { 28 }
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
    $current.millisecond = $current.millisecond + 1_000
    $current.second = $current.second - 1
    if $current.second < 0 {
        $current = (borrow-minute $from $current)
    }

    $current
}

def borrow-millisecond [from: record, current: record] {
    mut current = $current
    $current.microsecond = $current.microsecond + 1_000_000
    $current.millisecond = $current.millisecond - 1
    if $current.millisecond < 0 {
        $current = (borrow-second $from $current)
    }

    $current
}

def borrow-microsecond [from: record, current: record] {
    mut current = $current
    $current.nanosecond = $current.nanosecond + 1_000_000_000
    $current.microsecond = $current.microsecond - 1
    if $current.microsecond < 0 {
        $current = (borrow-millisecond $from $current)
    }

    $current
}

# Subtract two datetimes and return a record with the difference
# Examples
# print (datetime-diff 2023-05-07T04:08:45+12:00 2019-05-10T09:59:12+12:00)
# print (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
export def datetime-diff [from: datetime, to: datetime] {
    let from_expanded = ($from | date to-timezone utc | date to-record | merge { millisecond: 0, microsecond: 0})
    let to_expanded = ($to | date to-timezone utc | date to-record | merge { millisecond: 0, microsecond: 0})

    mut result = { year: ($from_expanded.year - $to_expanded.year), month: ($from_expanded.month - $to_expanded.month)}

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

    $result.millisecond = ($result.nanosecond / 1_000_000 | into int) # don't want a decimal
    $result.microsecond = (($result.nanosecond mod 1_000_000) / 1_000)
    $result.nanosecond = ($result.nanosecond mod 1_000)

    $result
}


