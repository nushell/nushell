# Spec for nu duration
:note: supersedes README-duration-spec0.md

Backing crate and Nu integration for an ergonomic and accurate duration and calendar module
## Scenarios

### Elapsed time
For perf measurements (needs efficiency) and also for "_units_-between", e.g **days**-between, **hours**-between.  
User specifies desired unit of measure, wants single quantity of those units. Perhaps an option to round vs truncate the quantity.

{days, weeks, months, quarters} between can be used for business or statistical analysis.
nanoseconds between can be used for performance reporting.

### Human-readable elapsed time
For reporting arbitrary durations, "it has been *year* years, *months* months, *days* days, ... *sec* seconds, *millisec* milliseconds ... since the first Nushell checkin".

User wants a **list** of time units.  User wants to specify which units are in the list (maybe skip "weeks"?) and how to handle the remainder for the last (least significant) unit on the list (show *sec*.*fraction* or just round to whole *sec*).

So above duration might be shown as:  
"*year* years, *months* months, *weeks* weeks, *days* days, ... *sec* seconds, *millisec* ms, *microsec* us, *nanosec* ns"   
or  
"*year* years, *months* months, (skip weeks) *days* days, ... *sec*.*fraction* seconds"   
or other combinations.

So there must be some kind of style option to control the calculation.

### Calendar arithmetic
"in 2 hours" -- does this mean now-down-to-the-nanosecond + 2 hours?  It could also mean "anytime between 2 and 3 hours from now"  
"one month from today" -- which has special semantics when today is at the end of the current month.  
"the last day of the month, year" -- same special semantics as above.  "first day of the month, year" has some special semantics, too.

Not just, though mosly, about dates.

So there needs to be a way to represent "one month from" but also "the {first,last} {timeperiod} of the {containing timeperiod}"

There also needs to be a way to **truncate** a duration to desired precision.  Like integer floor division?


### Iteration
"every Tuesday till the end of the quarter" -- simple repetition  
"the Tuesday after the first Monday in November" -- US elections.  

These build on [calendar arithmetic](#calendar-arithmetic).  I think the individual steps are covered by the above, but the iteration would be an additional feature, and might not be supported by the library.


## ways and means
### Considerations

The key point is there are 2 groups of durations that have to be supported:
* "daily" range -- (ns, us ... days, weeks)
* "monthly" range -- (months, quarters, years, .. centuries, millennia)

Within each range, there are simple multiplicative (though not linear) scaling factors.  But between the ranges, between days or weeks and months, all the vagaries of legislated calendars come into play, and the conversion is essentially table-driven.

Nu's current duration type supports the daily range.  How to add support for the "monthly" range? It could be an independent datatype, e.g `duration` and `calendar-duration`, with a couple of utilities to bridge the gap.  But, to take the analogy with `date` and `datetime`, Nu resolved to unify these into a single superset type.  So we resolve to provide a unified duration type that handles both duration ranges on the theory that it's easier for users (if it doesn't look like 2 types with the cracks between them papered over).

Regarding date/time representation, Nushell has standardized on [chrono](https://crates.io/crates/chrono)`::DateTime<FixedOffset>` for date/time.  Maybe the door is still open to standardize on the more compact, efficient and computationally tractable UTC time, with localtime I/O?  But for now, we'll support `DateTime<FixedOffset>`.

We assume users will not expect **leap second**s to be included.  Even if we did, best guidance from ISO is for implementors to only include historical, already-passed, leap seconds, not to try to include or predict any future ones.  Let that be someone else's problem.

Regarding calendars, the world has legislated (mostly) **solar** or **lunisolar** calendars (with a few pure-ish **lunar** calendars). We're only going to support the propelectic Gregorian calendar.  Somebody else can tackle the major **lunisolar** calendars.

Regarding overflows, they are likely because we're dealing with nanoseconds and the number of nanoseconds in a year is only a few orders of magnitude less than i64::MAX.  Overflow causes an unhandled panic and kill the shell, which is not friendly.

I considered using "saturating" operations, which would clamp at the upper or lower limit of the value instead of overflowing.  That works fine if the last operation in an expression is the one that saturates: the user will see 164::MAX or ::MIN, and can learn that's an error.  But if the saturation happens in some intermediate result, it will be consumed without error, producing an erroneous, but not manifestly nonsense result.   E.g `i64::MAX - i64::MAX == 0`, which might be close the result the user was expecting.

### Conclusions

* Resolved: there shall be a single Nu type which represents all durations.  The range of representable durations shall meet or exceed the duration from datetime::MIN to datetime::MAX.  
* Resolved: the duration shall be a **single** quantity and unit of time.  But for "calendar arithmetic" and "iteration",  it must be convenient to chain duration operations in a stable order, so you can represent "last day of next month" unambiguously.  "human readable" scenario shall be handled as a function returning a list of duration types.
* Resolved: there shall be duration constants that represent **whole** time units (1 day, 33 years...).
* Maybe: there shall be duration constants that represent **position within** a time unit (beginning of day, week, month)
* For integration into Nushell, date/times shall be [chrono](https://crates.io/crates/chrono)`::DateTime<FixedOffset>`, but the new duration type  `::Duration`
* Overflows and underflows will be avoided by using "checked" operations and will cause runtime errors rather than panics.
# API

There's 2 aspects here: an underlying Rust module/crate with types and methods for doing the operations; and the integration of the types into Nu parser and protocol.

chrono api, especially, is an odd mix of fallible and infallible operations (including some `Result<>`), also odd mix of signed/unsigned and variable word lengths.

NuDuration API should mostly use `Result<>`.  Errors should be boxed so they don't all have to be remapped (??).

Not sure about how to choose word sizes.

## Nu integration
The value and type shall be `Duration`, superseding the existing (more limited) duration type.

## Rust NuDuration module

### NuDurationUnit enum
Just the unit of measure (pure variant), 
Variants for:
* all the concrete units: ns, us, ms ... day, week, month, quarter, year, century, millennium  
  variant contains a single signed integer quantity
* remainder unit: `fraction`  
   stands for whatever's left over after humanizing a duration down to the smallest specified unit.  
   note quantity is a float with magnitude less than 1.  
   Not clear how to apply this in a chained operation being added to a duration. 
   You have to keep track of the previous unit of measure to apply the fraction.
  
* nested units: `first_{hour, day,week,month...}_of_{day, week, month...}`, 
  These refer to a unit relative to a containing unit.    
  The quantity is the number of time units to move (> 0, from start, < 0 from end), 
  :note: maybe this is better a chainable operation instead?  Too complex otherwise.

* day of week: `Sunday, Monday ... Saturday`, also numerical: `day_number_in_week`  
  the signed quantity is how many back or forward to go.
  These are special cases of the nested units above: a day within a week. 

Supporting tables / maps:
* acceptable UI aliases, 
* which is the canonical one, 
* singular and plural forms
* scaling divisor from "base" unit (either ns or day; I don't think we need pointer back to the base)
* 

### NuDuration struct
Contains:
* unit enum
* quantity of above. signed.
* (anything else?)

```
NuDuration {unit:NuDurationUnit, quan: isize}
```
Operations may use transient structs like chrono::Duration (a.k.a chrono::TimeDelta) to do calculations, but these are not
persisted in the struct.

NuDuration::MIN `<=` DateTime::MIN - DateTime::MAX (a very negative quantity)   
NuDuration::ZERO `==` 0_ns.   
NuDuration::MAX `>=` DateTime::MAX - DateTime::MIN   

#### Constructors

```
direct struct literal:
NuDuration {unit:NuDurationUnit, quan: isize}

NuDuration::new(NuDurationUnit, isize) -> NuDuration

NuDuration::from_iso8601(&str) -> Result<[NuDuration]>
    accepts a duration string like "PnYnMnDTnHnMn.nnnnnnnnS", returns a **list** of durations. 
    (standard doesn't have placeholders for milli- micro- or nano-seconds, uses fractional part instead.)
    Might also accept the "extended" form, "Pyyyy-mm-ddThh:mm:ss.fffffffff".

```

#### Operations
Every one of these operations is fallibe, per the supporting crates.
Requires catching the error and returning Value::Error all over the Nushell bindings.

Instead, plan to use "saturating" operations instead, reduce the amount of error handling required.
Of course, the saturated value becomes like `null` in SQL, a viral value infecting subsequent calculations.

```
max_ns, min_ns
max_<unit>, min_<unit> - representation of the saturating value returned instead of raising an error in Nu operations.


trait ChainDuration:: 
    <datetime>.add(<duration>) -> Result<datetime>
    (need a trait for this, so it can be a "method" of a datetime and support nice chaining)
    Do we need anything else?
    <duration> can be signed.
    chain of operations needs `?`.

<duration>.add(<duration>) -> Result<duration>
    Only supported in limited cases.  Base and added duration must be in the same range of durations, 
    i.e, both "day", or both "month" range.  "relative" durations can't be added at all.
    This can be bound to Nushell `+` operator, which can be fallible (unlike Rust, which can panic but not fail).
    In general, use NuDuration::apply() instead.

NuDuration::duration_diff(<start datetime>, <end datetime>, <unit of measure>) -> Result<NuDuration>
    <end> - <start> as a number of <unit of measure>.  The actual difference is **truncated** to that <unit of measure>.
    (note that it doesn't matter whether you subtract first and then truncate or truncate each addend then sum)
    
```

### Chain / list operations

Otherwise, these are associated functions of `NuDuration`.

In prior versions of Nushell, a group of durations was represented as a `record` with a field/value for each component duration.  In Nushell, this works because order of fields in a record is stable, but it's not a given in other languages.  Also, our use of singlular/plural units based on quantity makes the field names less manageable.

So in the new duration type, we will use a `list<duration>` to denote a sequence of durations, emphasizing that, for duration arithmetic, **order of operations matters**.

```
<duration>.normalize( base: DateTime<FixedOffset>, min_unit: NuDuration) -> [NuDuration]
    Returns list of NuDuration, in descending order by unit of measure, each successor is less than
    1 unit of its predecessor.

NuDuration::to_iso8601(&[NuDuration]) ->  String
    Inverse of [NuDuration::from_iso8601]
    Can be used with .normalize() to produce human readable normalized output, or with just a single duration
    to produce unnormalized output.
    TBD -- think about Serde 

NuDuration::apply(<start datetime>, &[NuDuration]) -> Result<datetime>
   Applies a list of supported transforms, in order.  Conceptually a `fold`, but with context keeping.
   Most operations add their (signed) duration.
   `frac` knows what the last applied unit of time was, so the fractional quantity is applied correctly.
   Nested unit durations should work fine so long as they're represented in a NuDuration.

NuDuration::duration_diff_list(<start datetime>, <end datetime>, &[<durations to include>], LSB_treatment::truncate, round, remainder (as separate duration))
    For the "human readable" scenario.


```
## Nushell bindings
### Constructors

Literals 
```
> let one_sec = 1_second
> let two_secs = 2_sec   # 'sec', 's', 'seconds' all are an alias for 'second'

> [$one_sec, $two_secs] 
[1_second, 2_seconds]    # stringifying a duration pluralizes the unit, where appropriate
```

Explicit initialization (for computed values)
```
> let two_mos = (2 | into duration --units 0_months)
> $two_mos
2_months

> let uom = 10_da   # quantity doesn't matter when duration used with --units, just the unit of measure
> let quan = 42
> let the_duration = ( $quan | into duration --units $uom)
> $the_duration
42_days
```
### Serialize

Stringify

``` 
> 4_mos
4_months

> 4_mos | into string     # into string doesn't yet handle duration
4_months
```
duration units will use the canonic name (not an alias) and will be pluralized when quantity is not +/- 1.

Differs from current nushell in a couple of ways:
* Current nushell doesn't use the underscore.  perhaps we need a configurable preference item to theme this?
* current nushell "normalizes" duration into bigger units, e.g:
  ```
  > 〉1033000000000ns
  17min 13sec
  ```
  new Nushell leaves quantity and unit of measure unaltered.

Binary
```
〉4_mos | into binary
Error: nu::shell::could_not_convert_duration_ns

  × Could not convert duration to nanoseconds: Incompatible time unit
   ╭─[entry #11:1:1]
 1 │ 4_mos | into binary
   ·   ─┬─
   ·    ╰── Incompatible time unit
   ╰────

〉5_ns | into binary
Length: 8 (0x8) bytes | printable whitespace ascii_other non_ascii
00000000:   05 00 00 00  00 00 00 00         
```
Binary number will be number of nanoseconds (as currently) and will *fail* if duration cannot be converted to nanoseconds.

The various `to` commands will use the stringified representation of duration where needed, no special handling required.

The changes to stringify will require breaking change notification (or extra code to gracefully handle reading in, e.g, a file with old format?)

### Deserialize
`into duration` will handle deserialization from string and binary.
```
〉"4_mos" | into duration
4_months

〉5_ns | into binary | into duration
5_nanoseconds
```

Currently, there's no way to deserialize "month" range durations from binary.  That's a bug!

### Parsing durations
To parse a duration into units and quantity:
```
> 80_days | into record
{unit: days, quantity: 80}
```

### to convert a duration into "human readable", normalized form
In current Nushell, this is a standard "stringification" operation (though not reversible).
However, to make it accurate when duration is a mix of >= months and <= weeks and also to provide flexibility in the string form, in the new Nushell, user will have to do a series of operations:
* compute the desired duration (often as a **list** of duration values)
* "normalize" the values (see `into duration --normalized` below)
* parse and format the result for presentation

e.g:
```
〉let epoch = ((date now) - 1970-01-01T00:00:00z)
〉$epoch
1686077974017025614_nanoseconds
〉let norm = $epoch | into duration --normalize
〉$norm
[53_years 5_months 5_days 20_hours 30_minutes 14_seconds 102_milliseconds 94_microseconds 230_nanoseconds]
〉$norm | each {into record | $"($in.unit): ($in.quantity)"} | str join ", "  # format 1
years: 53, months: 5, days: 5, hours: 20, minutes: 30, seconds: 14, milliseconds: 102, microseconds: 94, nanoseconds: 230
〉$norm | each {into record | $"($in.quantity) ($in.unit)"} | str join ", "   # format 2
53 years, 5 months, 5 days, 20 hours, 30 minutes, 14 seconds, 102 milliseconds, 94 microseconds, 230 nanoseconds



# note base_date is necessary to calculate months and years correctly

> (134_days + 22_min + 135_432_998_ns) | into duration --base-date 2022-10-31 --units 0_days
[4_months 2_weeks]
> (134_days + 22_min + 135_432_998_ns) | into duration --base-date 2022-10-31 --units 0_microseconds
[4_months 2_weeks 22_minutes 135_milliseconds 432_microseconds]

# for arbitrary roll-your-own flexibility in formatting a human-readable (list of) duration:
TBD

```
### Contexts assuming duration is int number of nanoseconds (legacy)

Prior versions of Nu stored durations as an `i64` number of nanoseconds.  As described above, this suffices for duration calculations up to units of weeks, but not months or more. However, the convention is pervasive, supported by `into int` and also `into binary`.

[[no! this is just weird when `9_ns / 3 -> 3_ns` and `4_days * 6 -> 24_days`.

For backward compatibility, then, in arithmetic operations involving duration and int or float, the duration will be converted to an equivalent number of nanoseconds, or throw an error if the duration is in months range.
... maybe not!]]

[[should `duration | into int --raw_duration` return the int quantity for any time unit?]]

[[also not sure we need this
```
> 3_months | into int --base-date 2022-10-31  # nanoseconds is the default unit
5_529_600_000_000_000
> 3_months | into int --base-date 2022-10-31 --units 0_days
64
> 2_hours | into int --units 0_sec
7200
```
]]
### Operations -- elapsed time scenario
Duration via subtraction, as heretofore.
```
> let start_time = (date now)
> sleep 3_sec
> let end_time = (date now)

> $end_time - $start_time
3_000_000_523_nanoseconds

> date diff $start_time $end_time  # nanoseconds is default for date diff, too.
3_000_000_523_nanoseconds

> date diff $start_time $end_time -- units sec
3_seconds
```
### Operations -- date difference for multi day durations
Duration via specialized command, with control of precision
```
> date diff 1492-10-12T13:03:58 (date now) --units 0_days
193808_days

# whereas nanoseconds would overflow

〉(date now) - 1492-10-12T13:03:58
Error: nu::shell::operator_overflow

  × Operator overflow.
   ╭─[entry #80:1:1]
 1 │ (date now) - 1492-10-12T13:03:58
   · ────────────────┬───────────────
   ·                 ╰── date subtract operation overflowed
   ╰────
  help: Consider using 'date diff --units' to perform the calculation with larger duration time units.
```
Result is truncated to an integer number of the units you request.
```
> date diff 1492-10-12T13:03:58 (date now) --units 0_months
6367_months

> date diff 1492-10-12T13:03:58 (date now) --units 0_years
530_years
```

This kind of result is useful for scheduling queries, for financial and business transactions (how much per day/week/month?).
Also useful for statistical grouping (count number of events per hour / day ...)

```
let events = [ <list of date/times of individual events of interest>]
let start = ($events | math min)
$events | each {|t| date diff $start $t --units 0_hours} | histogram
```

If you want to present a "humanized" result, you must perform an additional "normalize" operation:
```
> date diff 1492-10-12T13:03:58 (date now) --units 0_days | into duration --normalize --units 0_days
[530_years 7_months 25_days]
```
### Operations -- date plus duration
This and date difference are the key operations for duration data type.

In addition to fixed durations such as number of hours, minutes, days, there are calendar-aware durations such as months and years, but also "end of month" and "next tuesday".  In order to evaluate these durations user must provide a starting date to refer to.

Evaluating some calendar aware durations implies human-oriented interpretation of intent.

"a month from now" depends on the number of days in the current month, and is interpreted to require the result to be constrained to some day in the *following* month, regardless of its length.  

For days 1-28 of any month, this is easy (on gregorian calendar).
```
> 2023-01-01 + 1_month
2023-02-01
```

For days near the end of the current month, more "interpretation" is necessary:
```
> 2023-01-31 + 1_month
2023-02-28

> 2023-02-28 + 1_month
2023-03-31
```
One month from last day of this month should be last day of *next* month, regardless of how long or how short they are.

[[examples for "first of the (next) month", "end of the (current) month" and "next Monday"]]

These operations can be performed by arithmetic expressions, as shown above, but also by a specialized `date add` command which offers more flexibility (or complexity):

```
> 2023-01-31 + 1_month
2023-02-28

> 2023-01-31 | date add [1_month]
2023-02-28
```

The arithmetic forms preserve the time bits, though the calandar aware durations generally refer to a whole number of days.
But `date add` allows selective truncation or unit conversion if desired.

[[how? examples!]]

Similar considerations apply to "a year from now": the result should be the same day of the month, unless starting from the end of February in or to a leap year.

```
> let base_date = (date now)

> $base_date + 1_month
Fri, 30 Jun 2023 17:03:36 -0400 (in a month)

> $base_date | date add 1_month --units 0_day
Fri, 30 Jun 2023 00:00:00 -0400 (in a month)
```
### Operations -- chained operations
When doing arithmetic with day range durations (all of which can be converted to nanoseconds), the usual arithmetic operators work fine, and the following specialized operators don't improve accuracy (though they may avoid overflows).

But for scheduling or finance or other business calculations, the core scenario is adding a sequence of durations to a **given** date in order to calculate an **end** date. (or "subtracting a sequence of durations" to calculate a "starting" date). So you're essentially chaining the addition or subtraction operations to a date to produce another date

In these kinds of calculations,  **order of operations matters**.  

  2000-03-30 + 1mo + 1d -> 2000-05-01  
  2000-03-30 + 1d + 1mo -> 2000-04-30  
Duration addition is not commutative!  

(example courtesy of [dateutil package](https://github.com/hroptatyr/dateutils))

Although you can achieve the same results with standard arithmetic expressions and careful attention to detail, Nushell provides helpful specialized operations.

```
> let base_time = 2000-03-30
> $base_time + 1_day + 1_month
Sun, 30 Apr 2000 00:00:00 +0000 (23 years ago)

> $base_time | date add [1_day 1_month]
Sun, 30 Apr 2000 00:00:00 +0000 (23 years ago)
```

The list of durations preserves the order of operations and is an input or output of these functions.


### Operations -- date plus nested unit duration
A calendar arithmetic scenario: "a month from next Tuesday", "the first monday of next quarter"
```
> let base_date = (date now)

> $base_date | date add [1_Tuesday 1_month]
> $base_date + 1_tuesday + 1_month

> $base_date | date add [1_start_day_of_quarter 1_monday]
> $base_date + 1_start_day_of_quarter 1_monday

```

[end of document]