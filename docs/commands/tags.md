# tags

The tags commands allows users to access the metadata of the previous value in
the pipeline. This command may be run on multiple values of input as well.

As of writing this, the only metadata returned includes:

- `span`: the start and end indices of the previous value's substring location
- `anchor`: the source where data was loaded from; this may not appear if the
  previous pipeline value didn't actually have a source (like trying to `open` a
  dir, or running `ls` on a dir)

## Examples

```shell
> open README.md | tags
━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 span           │ anchor
────────────────┼──────────────────────────────────────────────────
 [table: 1 row] │ /Users/danielh/Projects/github/nushell/README.md
━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> open README.md | tags | get span
━━━━━━━┯━━━━━
 start │ end
───────┼─────
     5 │  14
━━━━━━━┷━━━━━
```

```shell
> ls | tags | first 3 | get span
━━━┯━━━━━━━┯━━━━━
 # │ start │ end
───┼───────┼─────
 0 │     0 │   2
 1 │     0 │   2
 2 │     0 │   2
━━━┷━━━━━━━┷━━━━━
```

## Reference

More useful information on the `tags` command can be found by referencing [The
Nu Book's entry on Metadata](https://www.nushell.sh/book/en/metadata.html)
