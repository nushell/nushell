# from ods

Parses OpenDocument Spreadsheet binary data into a table. `open` calls `from ods` automatically when the file extension  is `ods`. Use this command when `open` is unable to guess the file type from the extension.

## Examples

```sh
> open abc.ods
─────────────────
 Sheet1
─────────────────
 [table 26 rows]
─────────────────
```

```shell
> open abc.ods --raw
Length: 4816 (0x12d0) bytes
0000:   50 4b 03 04  14 00 00 00  00 00 00 00  00 00 85 6c   PK.............l
0010:   39 8a 2e 00  00 00 2e 00  00 00 08 00  00 00 6d 69   9.............mi
0020:   6d 65 74 79  70 65 61 70  70 6c 69 63  61 74 69 6f   metypeapplicatio
...
12a0:   00 61 10 00  00 4d 45 54  41 2d 49 4e  46 2f 6d 61   .a...META-INF/ma
12b0:   6e 69 66 65  73 74 2e 78  6d 6c 50 4b  05 06 00 00   nifest.xmlPK....
12c0:   00 00 06 00  06 00 5a 01  00 00 60 11  00 00 00 00   ......Z...`.....
```

```shell
> open abc.ods --raw | from ods
─────────────────
 Sheet1
─────────────────
 [table 26 rows]
─────────────────
```
