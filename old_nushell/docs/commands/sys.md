# sys

This command gives information about the system nu is running on.

## Examples

```shell
> sys
─────────┬─────────────────────────────────────────
 host    │ [row 7 columns]
 cpu     │ [row cores current ghz max ghz min ghz]
 disks   │ [table 4 rows]
 mem     │ [row free swap free swap total total]
 net     │ [table 19 rows]
 battery │ [table 1 rows]
─────────┴─────────────────────────────────────────
```

```shell
> sys | get host
──────────┬──────────────────────────────────────────────────────────────────────────────────────────────────
 name     │ Darwin
 release  │ 19.5.0
 version  │ Darwin Kernel Version 19.5.0: Tue May 26 20:41:44 PDT 2020; root:xnu-6153.121.2~2/RELEASE_X86_64
 hostname │ Josephs-MacBook-Pro.local
 arch     │ x86_64
 uptime   │ 5:10:12:33
 sessions │ [table 2 rows]
──────────┴──────────────────────────────────────────────────────────────────────────────────────────────────
```

```shell
> sys | get cpu
─────────────┬────────
 cores       │ 16
 current ghz │ 2.4000
 min ghz     │ 2.4000
 max ghz     │ 2.4000
─────────────┴────────
```

```shell
> sys | get mem
────────────┬─────────
 total      │ 68.7 GB
 free       │ 11.1 GB
 swap total │ 0 B
 swap free  │ 0 B
────────────┴─────────
```
