# sum

This command allows you to calculate the sum of values in a column. 

## Examples

To get the sum of the file sizes in a directory, simply pipe the size column from the ls command to the sum command.

```shell
> ls | get size | sum
━━━━━━━━━
 value
━━━━━━━━━
 51.0 MB
━━━━━━━━━
```

Note that sum only works for integer and byte values at the moment, and if the shell doesn't recognize the values in a column as one of those types, it will return an error.

```shell
> open example.csv
━━━┯━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━
 # │ fruit   │ amount │ quality
───┼─────────┼────────┼──────────
 0 │ apples  │ 1      │ fresh
 1 │ bananas │ 2      │ old
 2 │ oranges │ 7      │ fresh
 3 │ kiwis   │ 25     │ rotten
━━━┷━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━
```

```shell
> open example.csv | get amount | sum
error: Unrecognized type in stream: Primitive(String("1"))
```
