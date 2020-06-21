# date

Use `date` to get the current date and time. Defaults to local timezone but you can get it in UTC too.

## Flags

    --utc
      Returns the current date and time in UTC

    --local
      Returns the current date and time in your local timezone

## Examples

```shell
> date
──────────┬────────
 year     │ 2020
 month    │ 6
 day      │ 21
 hour     │ 18
 minute   │ 3
 second   │ 43
 timezone │ -04:00
──────────┴────────
```

```shell
> date --utc
──────────┬──────
 year     │ 2020
 month    │ 6
 day      │ 21
 hour     │ 22
 minute   │ 3
 second   │ 53
 timezone │ UTC
──────────┴──────
```

```shell
> date --local
──────────┬────────
 year     │ 2020
 month    │ 6
 day      │ 21
 hour     │ 18
 minute   │ 4
 second   │ 3
 timezone │ -04:00
──────────┴────────
```
