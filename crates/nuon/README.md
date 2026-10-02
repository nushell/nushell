Support for the NUON format: [`from_nuon`] deserializes it, [`to_nuon`] serializes it.

The NUON format is a superset of JSON designed to fit the feel of Nushell.
Some of its extra features are
- trailing commas are allowed
- commas are optional in lists
- quotes are not required around keys or any _bare_ string that do not contain spaces or special characters
- comments are allowed, though not preserved when using [`from_nuon`]
- numbers may be hexadecimal, octal or binary, carry `_` separators, or lead with `+` or a bare `.`
- durations, filesizes and datetimes are literals: `2min`, `1kb`, `2000-01-01T00:00:00+00:00`
- so are binary, cell-paths and ranges: `0x[be ef]`, `$.a.b`, `1..5`
- a list of uniform records may be written as a table, which names its columns once rather than
  once per row

## Example
below is some data in the JSON format
```json
{
    "name": "Some One",
    "birth": "1970-01-01",
    "stats": [
      2544729499973429198,
      687051042647753531,
      6702443901704799912
    ]
}
```

and an equivalent piece of data written in NUON
```nuon
{
    name: "Some One",       # the name of the person
    birth: "1970-01-01",    # their date of birth
    stats: [                # some dummy "stats" about them
      2544729499973429198,
      687051042647753531,
      6702443901704799912, # note the trailing comma here...
    ], # and here
} # wait, are these comments in a JSON-like document?!?!
```

## Tables

Once there is more than one record, JSON has to repeat every key on every row:

```json
[
  {
    "name": "Erwin",
    "birthday": "October 14th",
    "height_cm": 188,
    "status": "based"
  },
  {
    "name": "Levi",
    "birthday": "December 25th",
    "height_cm": 160,
    "status": "shrimp"
  },
  {
    "name": "Reiner",
    "birthday": "August 1st",
    "height_cm": 185,
    "status": "grief"
  }
]
```

NUON has a table form, which names the columns once and then lists the rows. This is the same
data:

```nuon
[
  [name,   birthday,        height_cm, status];
  [Erwin,  "October 14th",  188,       based],
  [Levi,   "December 25th", 160,       shrimp],
  [Reiner, "August 1st",    185,       grief]
]
```

Twenty lines become six, and the four column names are written once instead of three times. With
three rows that is a small saving. With three hundred it is most of the file.

The names and statuses are unquoted because nothing in them forces quoting. The birthdays are
quoted because they contain a space and a digit. The writer decides that per string.

## Everything at once

Every bit of syntax NUON adds, in one kitchen sink example:

```nuon
{
    # comments
    unquoted: bare_strings_need_no_quotes,
    single_quotes: 'I can use "double quotes" here',
    backticks: `also literal, no escapes`,
    raw_strings: r#'no escapes in here, so \n stays two characters'#,
    hexadecimal: 0xdecaf, octal: 0o755, binary: 0b1011,
    leading_decimal_point: .8675309, and_trailing: 8675309.,
    positive_sign: +1,
    digit_separators: 1_000_000,
    not_finite: [inf, -infinity, nan],
    duration: 2min, filesize: 1kb,
    datetime: 2000-01-01T00:00:00+00:00,
    binary_literal: 0x[be ef],
    cell_path: $.a.b,
    range: 1..5,
    no_commas_needed: [these are separated by spaces],
    trailing_comma: [in, lists, too,],
    "backwards_compatible": "with JSON",
    table: [[name, age]; [Alice, 30], [Bob, 25]],
}
```

## Specification

The list above is a summary, not a definition. The spec covers the format in enough detail to
write a parser and a writer from. It describes how NUON should behave; `bugs_to_fix.md` tracks
where the current implementation still differs.

- [nuon_formal_specification.md](https://github.com/nushell/nushell/blob/main/crates/nuon/spec/nuon_formal_specification.md) - the format
- [bugs_to_fix.md](https://github.com/nushell/nushell/blob/main/crates/nuon/spec/bugs_to_fix.md) - where the current implementation still disagrees
  with the spec
