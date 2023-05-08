# New `Duration` type

Current Nushell `Duration` type has a couple of problems caused by core implementation choices.  This project attempts to address them and elevate
date and calendar arithmetic to first-class status in Nu.

Nushell currently has a `Duration` type separate from other numeric types, and literals to represent constant durations, such as `7.25day` or `22ns`.  

The internal representation of a duration is currently a number of nanoseconds. This can represent an interval of approximately 242 years (signed) and arithmetic on durations kept in nanoseconds is perfectly accurate within this range.  
However, Nu errs in offering a conversion from nanoseconds to larger duration units such as days or month *without* reference to the calendar or basing the calculation on a particular start date.  When Nu reports a duration in "months" or when you add a duration such as `1month` to a date, Nu is actually converting the month to 30 days' worth of nanoseconds, regardless of the number of days remaining in the current month or proximity to a leap day in a [leap year](https://en.wikipedia.org/wiki/Leap_year).  See examples of the problem in https://github.com/nushell/nushell/issues/8036

## Scenarios to cover
So these are the scenarios we're going to (try to) address:

### `<endTime> - <startTime>` -- elapsed time
  Frequent in performance measurement.  User wants maximum precision, and is likely to do follow-on calculations with the result, such as averaging.  Current Nu covers this pretty well, once you understand that `(<datetime> - <datetime>) | into int` is a number of nanoseconds.
  We don't want to break backward compatibility for this scenario.
### How many {hours, days ...} between `<startTime>` and `<endTime>` -- days (or hours ...) between
  This is a common calculation in financial or business calculations, also when counting events per hour, day... for statistical purposes.  This calculation has to be well-behaved for accounting purposes: when adding up a column of days between calculations, there must be no overlap or gap caused by rounding off higher precision hours, minutes seconds.   
  Conceptually, the calculation can be done by *truncating* each input to the desired time unit, then performing integer subtraction.  
  This is sometimes attempted in current Nu with an expression like: `(<endTime> - <startTime>) | into duration --convert days`, but the current bugs in duration block accounting accuracy.  For example, if `startTime` is after noon and `<endTime>` is before noon, the resulting duration is less than a day's worth of nanoseconds, so the right thing to do is round *up*.  But if startTime is before noon and endTime is after noon, the resulting duration should be rounded *down*, and it takes sophisticated application logic to ensure the correct answer.
### `<dateTime> plus-or-minus <duration>` - "the {day} a {week, month, year} from now" 
  Nu currently gets the calculation right, but only if user specifies `<duration>` as a literal with units less than a month (to avoid the 30-day-per-month assumption).  Also the calculation includes the time portion of the `datetime` which can produce suprising results when chained.  Nor is there a convenient way to round the `<datetime>` down to midnight.must be done by first truncating the time unit.  
  Here's a somewhat convoluted example:
  ```
〉let today = 2022-01-01T13:00:00
〉let tomorrow = $today + 1day
〉let day_after = $tomorrow + 1day
〉$day_after - $earlier_today
2day 13hr
〉let earlier_today = 2022-01-01T00:00:00
〉$today + ($day_after - $earlier_today)
Tue, 04 Jan 2022 02:00:00 +0000 (a year ago)

## expected something on 03 Jan 2022; I intended to add 1day, twice to 01 Jan.
```

Most commonly, you want a day date result.  Sometimes (when it's "any time next {week,month}", you want a week or even a month result).


### Rendering a duration for human consumption
  2 scenarios:
  * Rendering a Duration value during development or troubleshooting
    User expects accuracy, also full precision
    Nu currently has a bug: it wants to "reduce" or "normalize" a Duration value to a sum of values.  e.g:
  ```〉100day
  3month 1wk 3day
  ```
  However, in doing so it uses the non-calendar-sensitive algorithm, so a sum that includes "month" or more is likely to be incorrect.
  
  * Rendering a Duration value in applicaiton context
    Developer wants control over the formatting and precision, designed to suit the application
    e.g:
    ```
    It's been this long since Nushell's first commit: 3yr 12month 2day 20hr 55min 36sec 288ms 997µs 18ns
    ```
    (the weird display `3yr 12month 2day` rather than `4yr 2day` is logged as issue #9118)
    In other applications, developers might prefer to render this as:
    `4yr 3day` (with rounding) or `4.01yr` (fractional value, single time unit)
* Not-in-scope
  These scenarios are listed to *exclude* them from this project
  * Scheduling queries, like "first tuesday of every month", ...
  * Work week, working days calculations, like "how many working days till the end of the quarter"
  * ??

[[the intent is to use the scenarios above to justify and validate the design changes proposed below.  So add/refine the scenarios vigorously -- much better to have vigorous debate over the relevance of a scenario than the goodness of a design feature based on gut feel. ]]

## Design
So, how to fix this for Nushell, and make Duration useful and not misleading for the longer time intervals?  

The key is for the `Duration` type to contain a duration or time unit of measure *and* a quantity of those units, and for the operators and commands that deal with this type to perform calendar arithmetic in all the places it is required for accuracy.  

### Q&(some)A

* Should quantity part of Duration be `int` or `float`?
  Resolved: should be `int`
  * Pros: no roundoff error, good for accounting accuracy; for larger units, like `3.5_months` does not beg the question of "how many days in a month?"
  * Cons: forces user to choose the precision required in each calculation.  If you ask for `days` you get a result that is rounded down to whole days and lose precision.
* Should we be able to convert a duration into arbitrary other units?
  Resolved: no.  We'll do `<duration> <plus or minus> <duration> *only* for cases that don't need calendar arithmetic.* Pros: Doing the calendar arithmetic means associating a reference *date* with the duration.  Declaring it's just not supported means simpler implementation.
  Cons: Somewhat arbitrary rules for user to learn, fiddling error messages to test for prohibited operations.  Risk that there's some important scenario where this is really needed.  Until then, resist the temptation.
* Even though Value::Duration.quantity is int, should we allow literal where quantity is a decimal?
  Can do this safely when units are less than month.
  Pro: allows user a concise and convenient shorthand, though only in limited range
  Con: Being familiar with entering literals this way, user might be suprised to see integer calculations (and truncation)being done on Duration valued expressions.


## Goals by example

This is the new syntax and results we want to enable that we believe cover the above scenarios.  They should be usable as unit tests when the project is done.

[[Note: examples use UTC dates, for simplicity.  and the year 2020 *was* a leap year]]

[[credit: ttps://www.timeanddate.com for the detailed calendar arithmetic]]
### Elapsed time, in nanoseconds
```
〉(date now) - (date now)
-104_690_ns
〉(date now) - (date now) | into int
-15740
```
This is unchanged from current Nu, except that the rendering of the Duration result is reported in nanoseconds with `_` for readability.  It will not be normalized, even if a very large number.  

```
As heretofore, this calculation can overflow.

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

Date subtraction always returns durations in ns. If you want to get durations in larger units, such as days, see `date diff` command.

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
[[Aspirational example, doesn't work yet.]]

### Parsing a duration into quantity and type

If you want just the quantity (e.g for calculation), you must do an additional conversion:

```
> 22_days | into int
22

> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_day | into int
144
```

Another way to parse a Duration is to convert it to record.  
Duration can be converted into a record where units are the fields and quantities are the values.

```
> 2020-03-02T23:59:59.012_345_678 | date diff 2019_10_10T00:01:02 --unit 1_day | into record
{day:144}

> 2019_10_10T00:01:02 | date diff 2020-03-02T23:59:59.012_345_678  --unit 1_day | into record
{day:-144}
```
Unlike current Nu, the quantity can be a negative number, there is no `sign:` field.




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
Result is a date time and accurate.
[[ is the result *truncated* to the unit of duration?  Doesn't it have to be to handle 31-mar + 1_month?]]


### Normal rendering
Output of a duration value using the standard string format:
```
> 25_hr
25_hours
> 25_hour
25_hours
> 1_hours
1_hour
> 0_hour
0_hours
```
The literal allows multiple aliases for units, but the output always uses one canonic unit.  The unit is pluralized if the quantity is != 1.

The output is not converted to other time units, no matter how large the quantity is.

### Formatting duration 
Duration allows use of 



[[more examples]]




## Deliverables
(a SWAG)

1. (parser) A redefined `Duration` type which contains a unit of measure (nanoseconds through decades or centuries) and a quantity.  Extended use of `_` for readability in both duration literals and in duration.to_string(). Extended synonums for units of measure.
2. (parser) Duration explicitly supports  week, month, year... as legit units of measure.  (Directly fixes OP in #9028).  
3. (commands) An updated "humanized" representation for Duration.  The current one is somewhat pedantic and, er, inhumane.The updated representation can be converted back into an exact Duration.
4. (protocol) Arithmetic operators for mixed operations on datetime and duration.  These will handle durations with units greater than nanoseconds with full financial accuracy.  Note that operations involving 2 durations in general require a 3rd input, the base date.  These cannot be modeled with a binary operator, so special-purpose commands will cover them.
5. (protocol) Arithmetic operators involving durations with nanosecond units will *not* require new operators or special purpose commands and can be done as currently for backward compatibility.
6. (commands) Special purpose commands to perform mixed date and duration arithmetic operations that canot be modelled as binary operations.   These commands will have flags to all full control over the operation.
10. This represents kind of a big change to date and duration arithmetic.  Needs content for the book.
1. Other stuff we'll find out along the way.

## Design
In the new work, we will ignore the [leap second](https://en.wikipedia.org/wiki/Leap_second).  The Rust `chrono` crate (and most other calendar libraries) to not support and have provided pragmatic justification for that choice.  Nu should document the restriction and move on.

### Duration type -- in the parser
The type will store both a unit of measure and an *integer* quantity of those units. (today, Nu stores an integer number of nanoseconds). 

Format of literal will allow an optional `_` for readability:  Not only `<quan><unit>`, like `3sec`, as currently but also `<quan>_<unit>`, like `3_year` or `4_ns`.  (cf #9111, which requests this in additional literals).

When a Duration is displayed as result, or otherwise converted to string representation (Display trait), the unit of measure is the unabbreviated (canonic) one and will *include* the underscore.  So these input literals: `3da`, `3days` `3_day` will all output as `3_days`.  Likewise `1da` and `1day`, even `1days` will all output as `1_day`.

[[open question:
Numeric part of duration can be specified as a decimal number (not arbitrary float)]] if the units are weeks or less (parse error if too big a unit).  The actual value stored is the corresponding number of nanoseconds, or an error if it overflows. [[reason: 2.5_days is each to convert to 60_hours, so why not just use the next smaller unit?  but 3.1415926999_days would still be a decimal number of hours, and it seems excessive to iterate down through the units till you find one that doesn't lose precision.]]


### DateTime type -- in the parser
[[ Some nits identified for DateTime.  Move into separate PR?]]

Potential enhancements:
* Use  `_` for readability in the nanoseconds portion of a datetime.  Or cover as part of #9111?

   Example:
   ```
   # works today:
   〉2022-10-03T10:03:01.500333222
   Mon, 03 Oct 2022 10:03:01 +0000 (7 months ago)

   # would work in future:
   〉2022-10-03T10:03:01.500_333_222
   Mon, 03 Oct 2022 10:03:01 +0000 (7 months ago)
   ```
### Duration arithmetic -- crates/nu-protocol/src/value/mod.rs
Binary operations involving a date and a duration can be done simply (assuming you have a good calendar)
   * datetime plus duration -> datetime, duration plus datetime ==> datetime  
   * datetime minus duration -> datetime  

Difference between 2 dates produces a duration, but you must specify the units for the resulting duration in the general case.  However, calculating elapsed time is a very common scenario, and there you can assume high precision (nanoseconds). 
   * datetime minus datetime -> `duration<nanoseconds>`
   This is what the standard binary minus should do.  It can overflow if > 2^^61 ns (73.07 yr).  
   For date difference when you want to specify result units, see `date diff` below.

   * duration minus duration, duration plus duration   `->duration<>`
   This one is straightforward if units for both sides are the same, e.g seconds and seconds.  But if they're not, we disallow (and force the user to convert them to equal units via `<duration> | into duration --units`.  The error message should say so.)
### `<datetime> | date diff  <base_datetime> --units <unit> -> duration<unit>`
Subtracts two dates, returns Duration in requested units
### `<duration> | duration diff --relative-to <date> --units <unit> -> duration<unit>`
FM! What does this do?  Do we need a base date for each duration?
### `<duration> | into duration --units <unit>` for duration conversion
deprecate `--convert <unit>`, result of the above is Duration, not string.  
[[needs work, overlaps into record and cant be done for unit > week]]
### `<duration> | into record --relative-to <date> --compound -> record<>`
This signature already exists, but is being repurposed.
By default, this can be used to parse a duration into quantity and units.  The output record has only 1 field, whatever unit was in the Duration, and --relative-to is not needed.

If both --relative-to and --compound (is this the right word?) are specified rerturns a record with a sum of durations which is exact.  (currently this command returns record like `{days:N hours:N2 minutes:N3...}`, that will be extended)
It also adds a `basedate: <date>` to output record, so result can be converted back to  an accurate Duration. [[This makes the record serialization of a Duration a bit more expressive than `Type::Duration`.  Is this a problem?]]

### `<record<>> | into duration --units <unit> -> <duration<unit>>`
This signature already exists.  
Inverse of above.  If record is compound, it must also include the `basedate` field.
`--units` flag specifies desired units of the duration.
### `OneOf(<int>, <float>) | into duration --units <unit> -> duration<unit>`
You know what this does.
### `<duration> | into int -> int `
Returns the whole number of units in the duration.
### `into duration` and `into string` for serde (and humanize)
[[WIP, want to be able to produce really humane output and be able to convert it back]]
[[more here]]

### Rounding and truncating, dates and durations
[[I think these are covered by converting to record then munging that.]]
