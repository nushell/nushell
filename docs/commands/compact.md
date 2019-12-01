# compact

This command allows us to filters out rows with empty columns. Other commands are capable of feeding `compact` with their output through pipelines.

## Usage
```shell
> [input-command] | compact [column-name]
```

## Examples 

Let's say we have a table like this:

```shell
> open contacts.json
━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━
 # │ name     │ email
───┼──────────┼──────────────────
 0 │ paul     │ paul@example.com
 1 │ andres   │
 2 │ jonathan │
━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━
```

`compact` allows us to filter out rows with empty `email` column:

```shell
> open contacts.json | compact email
━━━━━━┯━━━━━━━━━━━━━━━━━━
 name │ email
──────┼──────────────────
 paul │ paul@example.com
━━━━━━┷━━━━━━━━━━━━━━━━━━
```
