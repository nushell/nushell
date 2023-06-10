# Spec for nu duration
;tl;dr

## deliverables checklist
Deliverables described in more detail below.

- [x] NuDuration datatype consists of unit + quantity
- [x] Parser recognizes NuDuration literals
- [ ] Operators
  - [ ] `<duration> cmp-op <duration>` works for all `> < >= <= ==`; gives "incompatible types" if both durations not in same range and not comparable by hueristic.
  - [ ] `<duration> cmp-op <number>` works for `<duration>` in days range, converting duration to ns; gives "incompatible types" error otherwise.
  - [ ] `<duration> plus-minus-op <duration> -> duration` works for both durations in *same* range, result is duration in lesser of the 2 units; gives "incompatibible units" error otherwise.  To add durations in different ranges, see
  - [ ] `<duration> div-mul-op <number> -> duration` works for any duration, result is duration in same units
  - [ ] `<datetime> plus-minus-op <duration> -> <datetime>` works for any duration.
  - [ ] `<datetime> minus-op <datetime> -> duration` works, result is nanoseconds; "overflow" error if not enough nanoseconds to express duration.  
- [ ] Command conventions -- apply to all the commands below
  - [ ] `--unit <duration_unit>` parameter accepts all the aliases supported by duration literals, or an actual duration value (quantity ignored)
  - [ ] `--base-date <datetime>` for specifying a base date in many of the commands below
- [ ] Conversions (`into xxx` commands)
  - [ ] all commands provide duration example (and unit test cases where appropriate)
    - [ ] `duration | into string` and `string | into duration`
    - [ ] `duration | into duration --unit <duration_unit> [ --base-date <datetime> ] -> <duration>`
    - [ ] `duration | into duration --normalize [ --unit-list <list<duration-unit> --base-date <datetime> ] -> list<duration>`   
        Normalize means to convert a given input to *list* of durations ordered biggest unit first.
    - [ ] `duration | into record -> record<quantity:int unit:string>`

    - [ ] `duration | into binary` and `binary | into duration`
      - [ ] error if not days range duration (convertible to NS)
      - [ ] some way to serde month range durations
    - [ ] `duration | into int` and `int | into duration`
      - [ ] error if not days range duration (convertible to NS)
      - [ ] some way to serde month range durations
    - [ ] `duration | into decimal` and `decimal | into duration`
      - [ ] error if not days range duration (convertible to NS)
      - [ ] some way to serde month range durations
  - [ ] Formats (`to` / `from` commands)
    - [ ] Verify they handle duration by to_string() / try_from_string()
- [ ] `<end_datetime> | date diff --base-date <start_datetime> [ --unit <duration_unit>] -> <duration>`
  - [ ] for `--unit` in months range, handles end-of-month, -quarter, -year [heuristics](#adding-month-quarter-and-year-durations-to-date)
- [ ] `list<duration> | date add --base-date <start_datetime> [ --unit <duration_unit>] -> list<datetime>`
  - [ ] end-of-month, -quarter, -year [heuristics](#adding-month-quarter-and-year-durations-to-date)

## Non-deliverables:
These are not part of this project, though some of them could be good add-ons later
* "compound" durations: "nth &lt;smaller unit&gt; of the "mth" &lt;containing unit&gt;", e.g "3rd day of next month"  This includes "next tuesday" (second day of the next week).  These would be very handy for schedules and for date iteration.
## Key concepts:
The new duration `Value::` is called, naturally, `NuDuration` both here and in the code.  But the user-visible `Type::Duration` and the name as it appears in signatures remains `duration`.
### "day" range vs "month" range durations
Duration can now represent nanoseconds through weeks (the "day" range) and also months through millennia ("months" range).  

It is not possible to add or subtract durations from different ranges exactly, becuause of the variable number of days in a month, or, to a lesser extent, the variable number of days in a (leap) year.  It is likewise not always possible to *compare* durations from different ranges, although hueristics can be applied to do cross-range comparisons for much of the duration axis.

The user *can* recover full comparison and arithmetic operation capability by providing a specific base date: then the calculation can be done by first adding each duration to the base date and either comparing the resulting dates or subtracting them to produce a duration.
### duration quantity as an int, with truncation
A NuDuration has an **integer** quantity and an enum unit-of-measure.  Although fractional quantities may be specifiable in some expressions, the resulting duration quantity is **truncated** in the result.  And the user generally must specify what units the result should have.  This allows duration arithmetic to be exact, with no comparison epsilon or roundoff error.
d### legacy: duration as nanoseconds
Legacy was to treat duration as an int # nanoseconds, used in comparison and arithmetic operators, the conversion commands (`into`) and format (`to` and `from`).  Now that duration can also be an (incommensurate) number of months, how to proceed? 
* format commands serde a string representation of duration with unit (e.g "22_nanoseconds") rather than an int number of nanoseconds (0x0000000000000016).  This is a breaking change, if user  has stored the results of an old `to json` in a file and is now trying to read it back.
* conversion commands `into binary / int / decimal` will accept day range durations and convert to nanoseconds, as before.  They will issue error "incompatible value" if duration is in months range.
  * open question: if `duration(ns) | into int` returns nanoseconds, should not `to binary` as well?
  * open question: should these commands have an alternative mode in which they all simply serde the numeric quantity of any kind of duration?  e.g `<duration> | into int --raw`.  Would this be simply too confusing if they can also process nanoseconds?
### adding month, quarter and year durations to date
"a month from now" depends on the number of days in the current month, and is interpreted to require the result to be constrained to some day in the *following* month, regardless of relative lengths of current and following month.  

For days 1-28 of any month, this is easy (on gregorian calendar).
```
> 2023-01-01 + 1_month
2023-02-01
```
For days near the end of the current month, more "interpretation" is necessary:
```
> 2023-01-31 + 1_month  # current month longer than next month and date "near" end of month
2023-02-28              #   -> truncate to end of next month

> 2023-02-28 + 1_month  # current month shorter than next month and date *at* end of month
2023-03-31              #   -> **pad** to end of next month
```

Similar heuristics apply to:
* calendar "quarters" -- the day near / at end of quarter compared to corresponding day of following quarter (due to variable number of days in 3 months of the quarter)
* year -- last day of this year vs last nay of next year, due to variable number of days in the year (leap year)

## Open questions:
TBD

## Scenarios
(using new syntax, these can become examples or, at least, test cases)
### Elapsed time for perf measurement
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
### Number of days between start and end
Also a duration or elapsed time calculation, but needs the result in specific units of time.
The usual framing is, "how many days **between** start and end", result should be **truncated** to a number of days.

Useful for financial and business applications
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

### Elapsed time for statistical buckets
To count the number of events that occur in the same minute, hour, day...

Given a series of event datetimes, turn each into a duration from some standard starting datetime, 
then group the durations by minute, hour, day...
```
let events = [ <list of date/times of individual events of interest>]
let start = ($events | math min)
$events | each {|t| date diff $start $t --units 0_hours} | histogram
```

### Human-readable elapsed time
For reporting arbitrary durations, "it has been *year* years, *months* months, *days* days, ... *sec* seconds, *millisec* milliseconds ... since the first Nushell checkin".

```
〉let epoch = ((date now) - 1970-01-01T00:00:00z)
〉$epoch
1686077974017025614_nanoseconds
〉let norm = $epoch | into duration --normalize --base-date 1970-01-01T00:00:00z
〉$norm
[53_years 5_months 5_days 20_hours 30_minutes 14_seconds 102_milliseconds 94_microseconds 230_nanoseconds]

〉$norm | each {into record | $"($in.quantity) ($in.unit)"} | str join ", "   # format 2
53 years, 5 months, 5 days, 20 hours, 30 minutes, 14 seconds, 102 milliseconds, 94 microseconds, 230 nanoseconds
```

Note the use of `duration | into record` to get quantity and units as int and string, respectively.

### days / weeks / hours from now; days / weeks / hours ago -- `<duration> + / - <datetime>`
Elapsed time is all about `<datetime> - <datetime>`.  This scenario is all about `<duration> plus-minus-op <datetime>` (or vice versa).

```
> (date now) + 45_minutes

Uses the [heuristics](#adding-month-quarter-and-year-durations-to-date) 


================================================

=================================================






## Spec notes
To be included in help and in the book.
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


# note base_date is necessary to calculate months and years correctly

> (134_days + 22_min + 135_432_998_ns) | into duration --base-date 2022-10-31 --units 0_days
[4_months 2_weeks]
> (134_days + 22_min + 135_432_998_ns) | into duration --base-date 2022-10-31 --units 0_microseconds
[4_months 2_weeks 22_minutes 135_milliseconds 432_microseconds]

# for arbitrary roll-your-own flexibility in formatting a human-readable (list of) duration:
TBD

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