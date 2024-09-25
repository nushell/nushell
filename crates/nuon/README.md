Support for the NUON format.

The NUON format is a superset of JSON designed to fit the feel of Nushell.
Some of its extra features are
- trailing commas are allowed
- commas are optional in lists
- quotes are not required around keys or any _bare_ string that do not contain spaces or special characters
- comments are allowed, though not preserved when using [`from_nuon`]

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
