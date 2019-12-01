# format

Format columns into a string using a simple pattern

Syntax: `format <pattern>`

### Parameters

* `<pattern>`: the pattern to match

## Example

Let's say we have a table like this:

```shell
> open pets.csv
━━━┯━━━━━━━━━━━┯━━━━━━━━┯━━━━━
 # │ animal    │ name   │ age
───┼───────────┼────────┼─────
 0 │ cat       │ Tom    │ 7
 1 │ dog       │ Alfred │ 10
 2 │ chameleon │ Linda  │ 1
━━━┷━━━━━━━━━━━┷━━━━━━━━┷━━━━━
```

`format` allows us to convert table data into a string by following a formatting pattern. To print the value of a column we have to put the column name in curly brackets:

```shell
> open pets.csv | format "{name} is a {age} year old {animal}"
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ <value>
───┼─────────────────────────────────
 0 │ Tom is a 7 year old cat
 1 │ Alfred is a 10 year old dog
 2 │ Linda is a 1 year old chameleon
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```