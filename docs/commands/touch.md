# touch

Create one or more files in the current or an already existent directory.
It has no effect on existing files.
Unlike GNU touch, the access time and the modified time are not updated.

`-h`, `--help`
  Display help message.

## Examples

Create a file in an empty folder. Then touch the file and list files again to observe that the modified time has not been updated.

```shell
> ls
> touch file.ext; ls
──────────┬─────────────
 name     │ file.ext
 type     │ File
 size     │ 0 B
 modified │ 0 secs ago
──────────┴─────────────
> touch file.ext; ls
──────────┬───────────
 name     │ file.ext
 type     │ File
 size     │ 0 B
 modified │ 10 secs ago
──────────┴───────────
```

Create a file within an already existent folder.

```shell
> mkdir dir
> touch dir/file.ext; ls dir
──────────┬───────────
 name     │ dir/file.ext
 type     │ File
 size     │ 0 B
 modified │ 0 secs ago
──────────┴───────────
```

Create three files at once
```shell
> touch a b c
> ls
────┬────────────────────┬──────┬──────────┬──────────────
 #  │        name        │ type │   size   │   modified
────┼────────────────────┼──────┼──────────┼──────────────
  0 │ a                  │ File │      0 B │ 0 sec ago
  1 │ b                  │ File │      0 B │ 0 sec ago
  2 │ c                  │ File │      0 B │ 0 sec ago
────┴────────────────────┴──────┴──────────┴──────────────
