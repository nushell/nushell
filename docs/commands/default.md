# default

This command sets a default row's column if missing. Other commands are capable of feeding `default` with their output through pipelines.

## Usage
```shell
> [input-command] | default [column-name] [column-value]
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

`default` allows us to fill `email` column with a default value:

```shell
> open contacts.json | default email "no-reply@example.com"
━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━
 # │ name     │ email
───┼──────────┼──────────────────────
 0 │ paul     │ paul@example.com
 1 │ andres   │ no-reply@example.com
 2 │ jonathan │ no-reply@example.com
━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━
```
