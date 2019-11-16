# append
This command allows you to  append the given row to the table.

**Note**: 
- `append` does not change a file itself. If you want to save your changes, you need to run the `save` command
- if you want to add something containing a whitespace character, you need to put it in quotation marks

## Examples

Let's add more cities to this table:

```shell
> open cities.txt | lines
━━━┯━━━━━━━━━━━━
 # │ <value>
───┼────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
━━━┷━━━━━━━━━━━━
```

You can add a new row by using `append`:

```shell
> open cities.txt | lines | append Beijing
━━━┯━━━━━━━━━━━━
 # │ <value>
───┼────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
 4 │ Beijing
━━━┷━━━━━━━━━━━━
```

It's not possible to add multiple rows at once, so you'll need to call `append` multiple times:

```shell
> open cities.txt | lines | append Beijing | append "Buenos Aires"
━━━┯━━━━━━━━━━━━━━
 # │ <value>
───┼──────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
 4 │ Beijing
 5 │ Buenos Aires
━━━┷━━━━━━━━━━━━━━
```
