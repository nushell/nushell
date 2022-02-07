# rename

Use `rename` to give columns more appropriate names.

## Examples

```shell
> open /etc/passwd | lines | split column ":" | rename user password uid gid gecos home shell
────┬────────┬──────────┬──────┬──────┬────────┬─────────────────┬──────────────────
 #  │ user   │ password │ uid  │ gid  │ gecos  │ home            │ shell
────┼────────┼──────────┼──────┼──────┼────────┼─────────────────┼──────────────────
  0 │ root   │ x        │ 0    │ 0    │ root   │ /root           │ /bin/bash
  1 │ bin    │ x        │ 1    │ 1    │ bin    │ /bin            │ /usr/bin/nologin
  2 │ daemon │ x        │ 2    │ 2    │ daemon │ /               │ /usr/bin/nologin
  3 │ mail   │ x        │ 8    │ 12   │ mail   │ /var/spool/mail │ /usr/bin/nologin
────┴────────┴──────────┴──────┴──────┴────────┴─────────────────┴──────────────────
```
