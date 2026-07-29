def assert-duration-positive [value: duration metadata: record]: nothing -> duration {
    if $value <= 0sec {
        error make {
            msg: "Negative value passed when positive one is required"
            labels: [
                [text, span];
                ["use a positive value", $metadata.span]
            ]
            code: "nu::shell::needs_positive_value"
        }
    } else {
        $value
    }
}

def replace-timezone [tz: string]: datetime -> datetime {
	into record | upsert timezone $tz | into datetime
}

def extract-offset []: datetime -> duration {
	($in | replace-timezone "+00:00") - $in
}

# Round to the nearest specified datetime boundary before the input date.
@category date
@example "Round date up to nearest hour" "2026-07-15T12:11:10-04:00 | date floor 1hr" --result 2026-07-15T12:00:00-04:00
export def "date floor" [
    period: duration # Duration value representing the period boundary to which to round down to.
]: [
    datetime -> datetime
] {
    let input
    let $period = assert-duration-positive $period (metadata $period)

    let input_nanos = $input | into int
    let timezone: string = ($input | into record).timezone
    let offset_nanos = $input | extract-offset | into int

    let period_nanos = $period | into int

    $input_nanos - (($input_nanos + $offset_nanos) mod $period_nanos)
    | into datetime
    | date to-timezone $timezone
}

# Round to the nearest specified datetime boundary after the input date.
@category date
@example "Round date up to nearest hour" "2026-07-15T12:11:10-04:00 | date ceil 1hr" --result 2026-07-15T13:00:00-04:00
export def "date ceil" [
    period: duration # Duration value representing the period boundary to which to round up to.
]: [
    datetime -> datetime
] {
    date floor (assert-duration-positive $period (metadata $period))
    | $in + $period
}
