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

### Conclusions

* Resolved: there shall be a single Nu type which represents all durations.  The range of representable durations shall meet or exceed the duration from datetime::MIN to datetime::MAX.  
* Resolved: the duration shall be a **single** quantity and unit of time.  But for "calendar arithmetic" and "iteration",  it must be convenient to chain duration operations in a stable order, so you can represent "last day of next month" unambiguously.  "human readable" scenario shall be handled as a function returning a list of duration types.
* Resolved: there shall be duration constants that represent **whole** time units (1 day, 33 years...).
* Maybe: there shall be duration constants that represent **position within** a time unit (beginning of day, week, month)
* For integration into Nushell, date/times shall be [chrono](https://crates.io/crates/chrono)`::DateTime<FixedOffset>`, but the new duration type  `::Duration`



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

If we do saturating adds and subtracts, the saturation value reported will be the MAX or MIN, e.g (in Nushell) `max_ns`, `min_ns`.

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
> let the_duration = ( $quan | into duration --units $quan)
> $the_duration
42_days
```
### Serialize / Deserialize
In addition to literal and `to_string` described above, there are a couple of specialized functions
```
# to parse a duration into units and quantity
> 80_days | into record
{unit: days, quan: 80}

# to convert a duration into "human readable", normalized form
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

For backward compatibility, then, when referencing a new duration in a context expecting an int, a duration with units in the day range will be converted to nanoseconds but will issue a shell error if the duration is month range.

In addition, `into int` gets additional flags to do an explicit conversion:

```
> 3_months | into int --base-date 2022-10-31  # nanoseconds is the default unit
5_529_600_000_000_000
> 3_months | into int --base-date 2022-10-31 --units 0_days
64

```

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

```

Duration via specialized command, with control of precision
```
> let start_time = ('1492-10-12T13:03:58.012_345_678' | into datetime)
> let end_time = (date now) # today is 30-may-2023

> date diff $start_time $end_time --units 0_days
193808_days

> $end_time - $start_time
max_ns  # saturating add -- operation overflowed, but result is clamped to max or min instead of error
```

Normally, when you do a date diff operation, you want the result truncated to a particular precision
### Operations -- chained operations
You can apply a sequence of operations by simple arithmetic operations or specialized command.

Note that **order of operations matters**.  Although it's the `+` sign and `date add` subcommand, duration arithmetic is not commutative, though it is associative (I think).
```
> let base_time = ('2022-10-03T12:31:40' | into datetime)

> let end_time = $base_time + 1_sec + 1_month - 3_ns
> $end_time
(I have no idea)

> let end_time = ($base_time | date add [1_sec 1_month -3_ns])
> $end_time
(still no idea, but same as above)

> let end_time = $base_time + 1_start_day_of_month - 1_day   
> $end_time
Mon, 31 Oct 2022 00:00:00 -0400     # last day of month

> let end_time = ($base_time | date add [1_start_day_of_month -1_day])
> $end_time
Mon, 31 Oct 2022 00:00:00 -0400     # last day of month
```
The point of being able to apply a list of durations is that the lists work a bit like a closure, representing a deferred calculation that can be used on different inputs.

### Operations -- date plus duration
Many different scenarios here:  

"a month from now" (by which you mean, the day that is one month from now.)  
Or you might mean the exact nanosecond one month from now (or, more likely, one **hour** from now).

Typically, you'd simply add the duration to the date and get a full date and time.  
But if you're doing a sequence of these calculations and are concerned that rounding error might accumulate 
due to the time portion of these dates, you can truncate that off.
```
> let base_date = (date now)

> $base_date + 1_month
Fri, 30 Jun 2023 17:03:36 -0400 (in a month)

> $base_date | date add 1_month --units 0_day
Fri, 30 Jun 2023 00:00:00 -0400 (in a month)
```
Durations of a month can have unexpected results, because of the differing lengths of months (per calendar and per leap year)
Chrono interprets "end of the month" to mean "the last day of that month" and interprets month durations accordingly.
So when you ask for "a month from now", if it's 30-May, you'll get 30-June, which is not too surprising.  But if it's 31-May, the answer will also be 30-June, not 1-July (to keep the result within "the next month").  

If you ask for "a month from now" and it's 28-Feb in a non leap year, you'll get 31-March.  You were at the end of the current month, so you got the end of the next month.  If you ask for "28 days from now", you'll get 28-Mar.

Durations of a year are similarly affected by leap years. The details are left as an exercise for the reader, try asking for "a year from now" using start dates that will or won't include a leap year day.

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