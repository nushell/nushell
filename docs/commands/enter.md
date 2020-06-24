# enter

This command creates a new shell and begin at this path.

## Examples

```shell
/home/foobar> cat user.json
{
    "Name": "Peter",
    "Age": 30,
    "Telephone": 88204828,
    "Country": "Singapore"
}
/home/foobar> enter user.json
/> ls
━━━━━━━┯━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━
 Name  │ Age │ Telephone │ Country
───────┼─────┼───────────┼───────────
 Peter │  30 │  88204828 │ Singapore
━━━━━━━┷━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━
/> exit
/home/foobar>
```

It also provides the ability to work with multiple directories at the same time. This command will allow you to create a new "shell" and enter it at the specified path. You can toggle between this new shell and the original shell with the `p` (for previous) and `n` (for next), allowing you to navigate around a ring buffer of shells. Once you're done with a shell, you can `exit` it and remove it from the ring buffer.

```shell
/> enter /tmp
/tmp> enter /usr
/usr> enter /bin
/bin> enter /opt
/opt> p
/bin> p
/usr> p
/tmp> p
/> n
/tmp>
```

## Note

If you `enter` a JSON file with multiple a top-level list, this will open one new shell for each list element.

```shell
/private/tmp> printf "1\\n2\\n3\\n" | lines | save foo.json
/private/tmp> enter foo.json
/> shells
───┬────────┬─────────────────────────┬──────────────
 # │ active │ name                    │ path
───┼────────┼─────────────────────────┼──────────────
 0 │        │ filesystem              │ /private/tmp
 1 │        │ {/private/tmp/foo.json} │ /
 2 │        │ {/private/tmp/foo.json} │ /
 3 │ X      │ {/private/tmp/foo.json} │ /
───┴────────┴─────────────────────────┴──────────────
/>
```
