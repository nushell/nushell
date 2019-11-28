# split-row

Split row contents over multiple rows via the separator.

Syntax: `split-row <separator>`

### Parameters:
* `<separator>` the character that denotes what separates rows

## Examples

We can build a table from a file that looks like this

```shell
> open table.txt
4, 0, 2, 0, 7, 8

```

using the `split-row` command.

```shell
open table.txt | split-row ", "
━━━┯━━━━━━━━━
 # │ <value> 
───┼─────────
 0 │ 4 
 1 │ 0 
 2 │ 2 
 3 │ 0 
 4 │ 7 
 5 │ 8 
━━━┷━━━━━━━━━
```