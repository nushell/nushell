# New `Duration` type

Current Nushell `Duration` type has a couple of problems caused by core implementation choices.  This project attempts to address them and elevate
date and calendar arithmetic to first-class status in Nu.

Nushell currently has a `Duration` type separate from other numeric types, and literals to represent constant durations, such as `7.25day` or `22ns`.  

The internal representation of a duration is currently a number of nanoseconds. This can represent an interval of approximately 242 years (signed) and arithmetic on durations kept in nanoseconds is perfectly accurate within this range.  

However, Nu errs by freely converting from nanoseconds to larger duration units such as days or month *without* reference to the calendar.  When Nu reports a duration in "months" or when you add a duration such as `1month` to a date, Nu is actually converting the month to 30 days' worth of nanoseconds, regardless of the number of days remaining in the current month or proximity to a leap day in a [leap year](https://en.wikipedia.org/wiki/Leap_year).  See examples of the problem in https://github.com/nushell/nushell/issues/8036

A second issue pertains to usability of calculations involving larger durations, which often appear in financial or statistical applications.  [[more here]]

## Scenarios to cover
So these are the scenarios we're going to (try to) address:

(The intent is to use just these scenarios to validate the design changes proposed below, and not do design work that doesn't 
support one of these scenarios.  That way, we'll always know *why* we're adding a feature; we'll always be able to evaluate who would need it, and why.  
So, if your scenario isn't here or one of mine doesn't make sense to you, let's resolve things at that level before too much unhelpful code gets written.
The above applies to *features* users can see and use.  Implementation or architectural choices are *not* constrained by the above, and remain entirely up to the project team so long as they don't prevent delivering the features that cover the scenarios.)


### Calculating elapsed time
Frequent in performance measurement.  User wants maximum precision, and is likely to do follow-on calculations with the result, such as averaging.  

Current Nu covers this pretty well, once you understand that `(<datetime> - <datetime>) | into int` is a number of nanoseconds.

We don't want to break backward compatibility for this scenario.
### How many {hours, days ...} between `<startTime>` and `<endTime>`
Although it sounds a lot like elapsed time, it differs in critical ways. 
The user wants to know how many of his/her preferred time units intervene.  S/he understands that the answer is not as 
precise as possible and involves *truncation* back to the time unit involved (midnight of the day, 00 of the hour, day 1 of the month, etc).

This is a common calculation in financial or business calculations, where it is well-behaved for accounting purposes.  
You can add up a column of, e.g *day* differences and get a grand total in units of whole days, then you can group the values into several subtotals which you can then sum to get exactly the same grand total. This wouldn't be possible if the elements retained their fractional part: some subtotals would .  

This is sometimes attempted in current Nu with an expression like: `(<endTime> - <startTime>) | into duration --convert days`, but the current bugs in duration block accounting accuracy.  For example, if `startTime` is after noon and `<endTime>` is before noon, the resulting duration is less than a day's worth of nanoseconds, so the right thing to do is round *up*.  But if startTime is before noon and endTime is after noon, the resulting duration should be rounded *down*, and it takes sophisticated application logic to ensure the correct answer.
### `<dateTime> plus-or-minus <duration>` - "the {day} a {week, month, year} from now" 
Nu currently gets the calculation right, but only if user specifies `<duration>` as a literal with units less than a month (to avoid the 30-day-per-month assumption).  

Also the calculation includes the time portion of the `datetime` which can produce suprising results when chained.  Nor is there a convenient way to round the `<datetime>` down to midnight, though it can be done via substring on the to-string representation.

Here's a somewhat convoluted example:
```
〉let today = 2022-01-01T13:00:00
〉let earlier_today = 2022-01-01T00:00:00
〉let tomorrow = $today + 1day
〉let day_after = $tomorrow + 1day
〉$day_after - $earlier_today
2day 13hr
〉$today + ($day_after - $earlier_today)
Tue, 04 Jan 2022 02:00:00 +0000 (a year ago)

## expected something on 03 Jan 2022; I intended to add 1day, twice, to 01 Jan.
```
[[need a much better example to illustrate importance of truncation in date arightetic]]

### Rendering a duration to string
2 scenarios:
  * Rendering a Duration value via Display trait of `Value::DateTime`, as a result at the command line.  
    User expects accuracy, also full precision.  But nu currently wants to "reduce" or "normalize" a Duration value to a sum of values.  e.g:
    ```
    〉100day
    3month 1wk 3day
    ```
    ... And this is *always* the wrong answer.  There is no sequence on the calendar where 3 months in a row all have 30 days.
  
    Secondarily, Nu doesn't provide a convenient way to convert the normalized representation back into a Duration, though `<record> | into duration` might be pressed into service.
  
  * Rendering a Duration value in applicaiton context  
    Developer wants control over the formatting and precision, designed to suit the application
    e.g:
    ```
    It's been this long since Nushell's first commit: 3yr 12month 2day 20hr 55min 36sec 288ms 997µs 18ns
    ```
    (the weird display `3yr 12month 2day` rather than `4yr 2day` is logged as issue #9118)

    In other applications, developers might prefer to render this as:
    `4yr 3day` (with rounding) or `4.01yr` (fractional value, single time unit)

    It's not clear Nu needs a dedicated command to "humanize" a duration, if the date arithmetic tools produce reliable results, developer can craft his/her own custom format as needed.
### Not-in-scope
  These scenarios are listed to *exclude* them from this project
  * Caclulations involving day of week, like "first tuesday of every month"   
    Nothing here prevents adding day-of-week calculations later (hopefully).
  * Work week, working days calculations, like "how many working days till the end of the quarter"
  * ??


## Design

The key change is to remember the time *unit* along with the quantity in the `Duration` type, so when the time comes to count days in a month, you know exactly how many days the current duration represents.
The operators and commands that deal with `Duration` can then perform calendar arithmetic in all the places it is required for accuracy.  

### Open questions, some answers

* Should quantity part of Duration be `int` or `float`?  
  Resolved: should be `int`  
  * Pros: no roundoff error, good for accounting accuracy; with units >= "month", avoids the need to know how many days in 
   the intervening months.
  * Cons: forces user to choose the precision required in each calculation.  If you ask for `days` you get a result that is rounded down to whole days and lose hour, minute, sec... precision.
* Should we be able to convert a duration with one time unit into arbitrary other units?  
  Resolved: Yes, but at the expense of distinguishing cases that don't require a calendar from those that do.  See discussion of [`<duration> | into duration`](#duration--into-duration---units-unit---date-datetime-for-duration-conversion) for details.
  * Pros: Comprehsive arithmetic on Duration type.
  * Cons: Extra complexity and additional inputs for longer durations.
* Even though `Value::Duration.quantity` is `int`, should we allow literal where quantity is a decimal?  
  Resolved: open  
  Can do this safely when units are less than month, would have to issue error for bigger units.
  Pro: allows user a concise and convenient shorthand.
  Con: Being familiar with entering literals this way, user might be suprised to see integer calculations (and truncation)being done on Duration valued expressions.

* Which of the new/updated commands should vectorize?  
  Resolved: Yes, but how?  
  Durations and datetimes are simple scalars, most calculations probably should vectorize.  
  But I'm not clear on the pattern for implementing this in Nu, not sure how hard or easy it will be.  

## Goals by example

This is the new syntax and results we want to enable that we believe cover the above scenarios.  They should be usable as unit tests when the project is done.

[[Note: examples use UTC dates, for simplicity.  and the year 2020 *was* a leap year]]

[[credit: https://www.timeanddate.com for the detailed calendar arithmetic]]
### Normal rendering
Output of a duration value using the standard string format:
```
> 25_hr
25_hours
> -25_h
-25_hours
> 25_hour
25_hours
> 1_hours
1_hour
> 0_hour
0_hours
```
The literal accepts multiple aliases for units on input, but the Display trait output always uses one standard name for the unit.  The canonic spelling is pluralized if the quantity is != 1, so `1_day` but `2_days`.

The output is not converted to other time units, no matter how large the quantity is (but see `<duration> | into record --normalize`)

### Custom formatted rendering 
```
> let d = (2020-03-02T23:59:59.098_765_432 - date diff 2019_10_10T00:01:02)
> $d
2_527_937_098_765_432_nanoseconds
> let normalize_d = ($d | into record --normalize 1_sec)
> $normalize_d
{months:4, days:21 hours:23 minutes:58 seconds:57}  # min unit is sec.  Note: *truncated* not *rounded*
> $normalize_d | items {|unit, quan| $"($quan) ($unit)"} | str join ", "
4 months, 3 weeks, 3 days, 23 hours, 58 minutes, 57 seconds
```
### Elapsed time, in nanoseconds
```
〉(date now) - (date now)
-104_690_ns
〉(date now) - (date now) | into int
-15740
```
This is unchanged from current Nu, except that the rendering of the Duration result is reported in nanoseconds with `_` for readability.  It will not be normalized, even if a very large number.  

As heretofore, this calculation can overflow.

```
〉(date now) - 1492-10-11T00:00:00 
Error: nu::shell::operator_overflow

  × Operator overflow.
   ╭─[entry #94:1:1]
 1 │ (date now) - 1492-10-11T00:00:00 
   · ────────────────┬───────────────
   ·                 ╰── subtraction operation overflowed
   ╰────
  help: 
```

Date subtraction via minus always returns durations in ns. If you want to get durations in larger units, such as days, see `date diff` command.

### Difference between 2 dates, in desired units of duration
```
> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_day
144_days

> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_hour
3479_hours

```
The result is a duration.  Units of the duration are as requested, with higher precision (smaller units) *truncated*.

The type of `--units` is Duration.  The  units portion specifies the desired units of result, quantity is ignored.

This calculation can also be used to bucket a bunch of timestamped data into hour (or other unit) buckets

```
> let start = 2022-05-01
> open log.log | lines | 
  parse '{timestamp}: {other}` | 
  insert bucket { |r| ($r.timestamp date diff $start) --unit 1_hour} |
  histogram bucket
```
[[Aspirational example, doesn't work yet. Here's what the output should be...
```
╭────┬───────┬───────┬──────────┬────────────┬──────────────────────╮
│  # │ value │ count │ quantile │ percentage │      frequency       │
├────┼───────┼───────┼──────────┼────────────┼──────────────────────┤
│  0 │     9 │    20 │     0.20 │ 20.00%     │ ******************** │
│  1 │     4 │    14 │     0.14 │ 14.00%     │ **************       │
│  2 │    10 │    14 │     0.14 │ 14.00%     │ **************       │
│  3 │     5 │    10 │     0.10 │ 10.00%     │ **********           │
│  4 │     2 │     8 │     0.08 │ 8.00%      │ ********             │
│  5 │     6 │     7 │     0.07 │ 7.00%      │ *******              │
│  6 │     8 │     7 │     0.07 │ 7.00%      │ *******              │
│  7 │     0 │     6 │     0.06 │ 6.00%      │ ******               │
│  8 │     1 │     6 │     0.06 │ 6.00%      │ ******               │
│  9 │     7 │     5 │     0.05 │ 5.00%      │ *****                │
│ 10 │     3 │     3 │     0.03 │ 3.00%      │ ***                  │
╰────┴───────┴───────┴──────────┴────────────┴──────────────────────╯
```
]]



### Difference between 2 durations
We handle only the cases that do not require calendar arithmetic.
We can handle those in the context of plus and minus operators, do not need a `duration diff` command.
```
> 365_days - 1_day      # units of lhs and rhs match
364_days
> 365_days + 1_day      #  units of lhs and rhs match
366_days

> 365_days - 1_ns       # all cases where all inputs have units <= week
31_535_999_999_999_999_ns

```
When lhs and rhs units are the same, output has same units and it's a matter of combining the quantities (plus or minus)

When units are different, but all units are week or less, output has the units of the *smaller* input.

We convert the larger unit into smaller by multiplication (10^9 ns / sec; 60 s / min; 60 min / h; 24 h / da; 7 da / wk)
The calculation is accurate because we ignore the leap second, as noted [here](#design)

It doesn't matter if the effective duration of inputs or result is bigger than a month, so long as the *units* they are expressed in are less than a month, no calendar calculations are needed (because leap days consume a day on the number line and weeks are always 7 days)

When any unit is a month or more, this operation generates a helpful error.

If you want to do arithmetic on durations that do have units larger than a week, you will have to go back to the dates they were calculated from and do calendar arithmetic with `date diff` or possibly `<date> <plus or minus> <duration>`

### Date plus-or-minus duration

With the new duration correctly handling literals with units greater than a week, you can use arithmetic plus and minus operators for all date and duration operations.
```
<date> + <duration>
<date> - <duration>
<duration> + <date>
<duration> - <date>
```
Result is a date time.  Unlike `date diff`, result is not truncated to the units of `<duration>`. This is necessary to support the "date a year from now" scenario.  All arithmetic is done by calendar rules, which means adding `month` durations looks at the number of days in the resulting month.  

These end of month examples highlight the difference between Nu's current 1month == 30 days and calendar math.  For most days and most durations, the result matches your intuition.

```
# same day a year from now
> 2019-10-31 + 1_year
Saturday, October 31, 2020  

# adding one month means keeping the result within that month.
# e.g: October has 31 days, but November only 30, what is 1 month from the end of October?
> 2019-10-31 + 1_month
Saturday, November 30, 2019

> 2019-10-31 + 31_days    # if you want 31 days, ask for 31 days.
Sunday, December 1, 2019

# extreme end-of-month weirdness, involving shortest month (feb)
> 2019-01-30 + 1_month
Thursday, February 28, 2019
```

Subtracting a duration can also exhibit end-of-month weirdness:
(2020 is a leap year, but that doesn't affect end-of-month weirdness)
```
> 2020-03-31 - 1_month
Saturday, February 29, 2020   
> 2020-03-31 | date diff 2020-02-29 --units 1day
31day

but:
> 2020-03-28 - 1_month
Saturday, March 28, 2020
> 2020-03-28 | date diff 2020-02-28 --units 1day
29
```

### Parsing a duration into quantity and type

If you want just the quantity (e.g for calculation), you must do an additional conversion:

```
> 22_days | into int
22

> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_day | into int
144
```
If you don't know the units *a priori*, you can convert Duration into a record.
The units are the fields and quantities are the values.

```
> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_day | into record
{days:144}

> 2019_10_10T00:01:02 | date diff 2020-03-02T23:59:59.012_345_678  --unit 1_day | into record
{days:-144}
```
Unlike current Nu, the quantity can be a negative number, there is no `sign:` field.

Note that the field name will be pluralized if the quantity is not one (as an aid to human interface).

### Conversion between durations
`<duration> | into record` can convert between duration units to a limited extent (normalizing).
Is there a more general need to convert any given duration into any other given duration?
This is doable for units < month, as Nu does now.
Otherwise, I believe you need to specify a base date to do the calendar arithmetic correctly. e.g "30 months is how many days (relative to 2023-10-10)"

The command would be `<duration> | into duration --units <otherDuration> --date <datetime>`

But what's the scenario?


## Design
In the new work, we will ignore the [leap second](https://en.wikipedia.org/wiki/Leap_second).  The Rust `chrono` crate (and most other calendar libraries) to not support and have provided pragmatic justification for that choice.  Nu should document the restriction and move on.

## Deliverables
What's in the box?

1. (parser, protocol crates) A redefined `Value::Duration` type.  Explicitly supports  week, month, year... as legit units of measure.  (fixes OP in #9028).  
  At the moment, no changes seem to be needed for `Type::Duration`, `Shape::Duration`?
2. (protocol) Arithmetic operators involving durations (date - date, date +/- duration, duration +/- duration) will work for limited ranges, may raise errors outside that range.  Will support Nu current operations on durations in nanoseconds for backward compatibility.
3. (commands crate) Special purpose commands to perform additional date and duration arithmetic operations.   `date diff`, `into record`, `into duration`. Not planning on doing anything with `from` or `parse`.
4.  Update to website and the book to describe date/duration arithmetic.
5. Other stuff we'll find out along the way.
### Duration type -- in the parser
The type will store both a unit of measure and an *integer* quantity of those units. (today, Nu stores an integer number of nanoseconds). 

#### Time Units
Supported time units (and acceptable aliases) are:

| unit    |  plural   | aliases               |
|---------|-----------|-----------------------|
| year    |  years    | yr, yrs               |
| month   |  months   | mon, mons             |
| day     |  days     | da, das               |
| hour    |  hours    | h, hr, hrs            |
| minute  | minutes   | m, min, mins          |
| second  | seconds   | s, sec, secs          |
| millisecond | milliseconds | ms             |
| microsecond | microseconds | us, "\u{b5}s"  |
| nanosecond | nanoseconds | ns               |

Other time units are not supported: Century, Millennium.  These may all be readily (and accurately) computed from the above.

Any of the aliases are acceptable on input (in a duration literal).  When a duration is converted to string for output, 
The canonic unit name (left hand column) will be used, or its plural if the quantity is `!= 1`.

A duration literal consists of the quantity and time unit, optionally separated by an  `_` for readability.  The quantity 
can be signed.  The time unit can be any of the words in the table above.  The following forms all represent 3 minutes:
```
3m
3_m
3min
3_mins
3_minute
```

When a Duration is displayed as result, or otherwise converted to string representation (Display trait), the unit of measure is the unabbreviated (canonic) one and will *include* the underscore.  So all of the above examples will be converted to string 
```
3_minutes
```
for output.

Note on `--units <unit>` switch.  Several commands below have a `--units` switch used to specify desired time unit of output.  The `<unit>` argument can be:
* a string which is any of the names, plurals or aliases defined above
* [[maybe also?]] a `duration` literal, the units portion of which is used.  The quantity portion of the literal is ignored.

### Duration arithmetic -- crates/nu-protocol/src/value/mod.rs
Binary operations involving BinaryPlus, BinaryMinus and DateTime and/or Duration

* datetime minus datetime -> `duration<nanoseconds>`
 For date difference when you want to specify result units, see `date diff` below.

* datetime plus duration -> `datetime`, duration plus datetime -> `datetime`
* datetime minus duration ->  `datetime`
  Simple, given a calendar library.  :warn: `chrono` doesn't support adding durations in months or larger.  Hmmm?

* duration minus duration, duration plus duration  -> `duration`
   2 cases supported:
   1. If units of both inputs are < month, result will have units of smaller input (or should it just be limited to nanoseconds?)
   2. If units of both inputs are the same, result will have same units.  And units can be anything.This one is straightforward if units for both sides are the same, e.g seconds and seconds.  
   3. Otherwise, we disallow.  User can convert via  `<duration> | into duration --units --date`.  The error message should say so.



### `<datetime> | date diff  <base_datetime> --units <unit> -> duration<unit>`
Subtracts two dates, returns Duration in requested units
### `<duration> | into duration --units <unit> --date <datetime>` for duration conversion
Converts duration in one unit to equivalent duration in specified units.
  `--date` is optional, but if calendar is needed and it was omitted, operation will fail.
Calendar will be needed if units of `<duration>` and `<unit>` are different or either one is > month.


For the existing signature `duration | into duration -> duration`, deprecate switch `--convert <string>`, which returned string.  
For replacement, see [the example](#custom-formatted-rendering). 

Examples:
```
> 45_day | into duration --units 1sec
3888000_seconds

> 46_days | into duration --units 1sec
3974400_seconds
> 3974399_sec | into duration --units 1_day # truncate into whole days
45_days
```

### `<duration> | into record -> record<>`
Without the `--normalize` flag, which is described below, this command converts the duration to a record which looks like
`{<unit>: <quantity> . . .}`

`<quantity>` is a signed int.

`<unit>` is one of the supported [time units](#time-units).  It is pluralized if `<quantity>` is not 1.   

Matching a pattern for the field name is 
not possible in cellpath or `get`, so it's hard for the user to extract a particular field from the record by name.  What to do about that?  
But (I think) in most cases, user will be iterating through all fields, and won't need to get a particular one.

:note:  breaks backward compatibility -- format of output record changed.  No `sign:` field.

```
> 17_da | into record 
{days: 17}
```
This is useful for parsing a duration into quantity and units.

### `<duration> | into record --normalize <duration> -> record<>` 

Converts duration into so-called "normalized" form:
* The fields appear in order from biggest time unit to smallest (field order is stable in Nu records). 
* The smallest time unit in the record will have same units as `<duration>`.  (Quantity of `<duration>` is ignored)
* For a given field F1: quanF1 followed on the right by field F2: quanF2, the value quanF2 will always be a proper fraction of F1.  
  For example, in `{days: 15, hours: H}`, H < 24.
* Unused fields will be present with a zero value, e.g: `{days: 15, hours: 0, minutes:10}`  
  This maintains the expected ordering of fields in the record.

:note: breaks backward compatibility -- format of record is changed: no `sign:` field, units of duration spelled out, no `decade`.

Examples:
##### 
```
> let d = 2019-10-10T00:00:00+00:00
> $d + 17_day + 22_sec + 33_ns - $d | into duration 
{nanoseconds: 1468800000000}
> $d + 15_day + 48_hours + 22_sec + 33_ns - $d | into duration --normalize 1_sec
{days: 17, hours: 0, minutes: 0, seconds: 22}   # result is *truncated* to desired units
                                                # note hours normalized to days
                                                # note placeholder hours, minutes
```

### `record<> | into duration --units <unit> --date <date> -> duration<unit>`
[[maybe not needed??]]
Inverse of above.  A convenient way to turn a normalized record back into a duration.
But the signature is complicated, and user could get the same effect by:
```
> let d = 2019-10-10T00:00:00+00:00  # assume this is the base date for the following duration
> let n = (5_month + 17_day + 22_sec + 33_ns ) | into record --normalize
> $n
{months: 5, days: 17, hours: 0, minutes:0, seconds: 22, milliseconds: 0, microseconds: 0, nanoseconds: 33}
> mut new_d = $d
> $n | into record --normalize | items {|unit quan| $new_d = ( $new_d | date add ($quan | into duration --unit $unit)) }
> let new_n = $new_d - $d
$new_n
14_601_600_000000000  # hopefully

```



### `OneOf(<int>, <float>) | into duration --units <unit> -> duration<unit>`
You know what this does.
### `<duration> | into int -> int `
Returns the whole number of units in the duration.  Only returns nanoseconds if input duration was in ns.
[[is this a trap for user, source of silent bugs?]]

### `to nuon` and `from nuon` and other serde
These rely on the Display trait producing a string representation which is a valid literal of the same type.
That will be true of the new `Duration`, so perhaps no changes needed here.
Add a test case involving duration, if not there already, and see what happens.

