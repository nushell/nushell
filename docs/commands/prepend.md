# prepend
This command prepends the given row to the front of the table

**Note**: 
- `prepend` does not change a file itself. If you want to save your changes, you need to run the `save` command
- if you want to add something containing a whitespace character, you need to put it in quotation marks

## Examples

Let's complete this table with the missing continents:

```shell
> open continents.txt | lines
━━━┯━━━━━━━━━━━━━━━
 # │ <value>
───┼───────────────
 0 │ Africa
 1 │ South America
 2 │ Australia
 3 │ Europe
 4 │ Antarctica
━━━┷━━━━━━━━━━━━━━━
```

You can add a new row at the top by using `prepend`:

```shell
> open continents.txt | lines | prepend Asia
━━━┯━━━━━━━━━━━━━━━
 # │ <value>
───┼───────────────
 0 │ Asia
 1 │ Africa
 2 │ South America
 3 │ Australia
 4 │ Europe
 5 │ Antarctica
━━━┷━━━━━━━━━━━━━━━
```

It's not possible to add multiple rows at once, so you'll need to call `prepend` multiple times:

```shell
> open continents.txt | lines | prepend Asia | prepend "North America"
━━━┯━━━━━━━━━━━━━━━
 # │ <value>
───┼───────────────
 0 │ North America
 1 │ Asia
 2 │ Africa
 3 │ South America
 4 │ Australia
 5 │ Europe
 6 │ Antarctica
━━━┷━━━━━━━━━━━━━━━
```
