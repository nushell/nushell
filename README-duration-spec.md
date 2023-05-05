# New `Duration` type

Current Nushell `Duration` type has a couple of problems caused by core implementation choices.  This project attempts to address them and elevate
date and calendar arithmetic to first-class status in Nu.

Nushell currently has a `Duration` type separate from other numeric types, and literals to represent constant durations, such as `7.25day` or `22ns`.  

However, the internal representation of a duration is currently a number of nanoseconds.  This works well enough duration shorter than a day (if you ignore the [leap second](https://en.wikipedia.org/wiki/Leap_second), which most users, and most calendar libraries and crates, do).

If the duration is a day or more, accounting for the leap day in a [leap year](https://en.wikipedia.org/wiki/Leap_year) is considered essential to the accuracy of calculations and that changes the number of nanoseconds involved.

There is a second issue related to this: doing datetime calculations involving durations rounded to a specific unit of measure.  Examples: "how many calendar days between these two dates"? (frequent in financial and business calculations) or "count how many events occurred per day over the last year".  These involve calculating a duration between two date(time)s. But if you have already reduced the duration to a number of nanoseconds, you can never recover the number of hours or days without reference to one of the original date(time)s and redoing the calendar arithmetic.  Nushell `Duration` does not remember a base date for the duration value.

So, how to fix this for Nushell, and make Duration useful for the longer time intervals?  

The key is for the `Duration` type to retain the duration unit of measure as well as a quantity, like a Rust enumeration of tuple types.  
Given that, duration calculations (and rounding) can always be done accurately over the range of dates supported by the calendar. (it says here...)

## Deliverables
(a SWAG)

1. Redefine `Duration` types to contain a unit of measure (nanoseconds through decades or centuries) and a quantity.  Quantity is *signed*.
2. Parser work to recognize week, month, year... as legit units of measure.  (Directly fixes OP in #9028).  Proposed changes to `into` will handle the compound duration of the form currently returned by `into duration`, i.e "1wk 4day 6hr 7min".
3. Special case handling for arithmetic operators with Datetime and Duration:
These will all use calendar arithmetic to account for leap days properly.
4. Changes to `into duration` and `duration | into string`
5. Kind of a big change to date arithmetic.  Needs content for the book.

Other stuff we'll find out along the way.

## Design
### Duration type -- in the parser
Should quantity be float or int?  Float allows single unit of measure, e.g `4.352_years`.  Int would require a sum of Durations to express same value, the "pounds shillings pence" problem.  But int arithmetic is exact.  In any case, a signed quantity, so you can represent a negative duration.

How many units of measure? Parser should probably accept all commonly used terms: nanoseconds (no need for smaller units?) microseconds ... day, week, quarter, year ....  Internally, can collapse everything less than a day down to a number of nanoseconds. (counterexample?)

Syntax of literals can be same as currently: `<quan><unit>`, like 3sec.  Could also be extended to be `<quan>_<unit>`, like 3_year or 4_ns.  Nu numeric constants already allow `_` for readablilty.  (opened as #9111).

Parser work should accept more aliases for unit of measure, e.g `da`, `days`, not just `day`.  On output, these are all normalized to a canonic unit.
It would be nice if converting a Duration to string pluralized the units as necessary, `1_day` but `2_days`.
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
### ``<duration> | into int -> int `
Not needed, given <duration> into record?
### `into duration` and `into string` for serde (and humanize)
[[WIP, want to be able to produce really humane output and be able to convert it back]]
[[more here]]

### Rounding and truncating, dates and durations
[[I think these are covered by converting to record then munging that.]]
## Future examples
Given the design above, we should be able to support the following snippets.  These could be unit tests!

### Difference between 2 dates, in desired units of duration

### Difference between 2 durations

### Round-trip a humanized duration

### Rounding for bucketing of statistics


[[more examples]]