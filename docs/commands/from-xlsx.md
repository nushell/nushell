# from xlsx

Parses MS Excel binary data into a table. `open` calls `from xlsx` automatically when the file extension  is `xlsx`. Use this command when `open` is unable to guess the file type from the extension.

## Examples

```sh
> open abc.xlsx
─────────────────
 Sheet1
─────────────────
 [table 26 rows]
─────────────────
> open abc.xlsx --raw
Length: 6344 (0x18c8) bytes
0000:   50 4b 03 04  14 00 00 00  08 00 00 00  00 00 d5 5f   PK............._
0010:   a7 48 68 01  00 00 23 05  00 00 13 00  00 00 5b 43   .Hh...#.......[C
0020:   6f 6e 74 65  6e 74 5f 54  79 70 65 73  5d 2e 78 6d   ontent_Types].xm
...
18a0:   6b 73 68 65  65 74 73 2f  73 68 65 65  74 31 2e 78   ksheets/sheet1.x
18b0:   6d 6c 50 4b  05 06 00 00  00 00 0a 00  0a 00 7f 02   mlPK............
18c0:   00 00 33 16  00 00 00 00                             ..3.....
> open abc.xlsx --raw | from xlsx
─────────────────
 Sheet1
─────────────────
 [table 26 rows]
─────────────────
```
