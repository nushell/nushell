# debug

`debug` prints a debugging view of the table data. It is useful when you want to get the specific types of the data and while investigating errors.

## Examples

```
❯ ls | first 2 | debug
───┬──────────────────────────────────────────
 # │ <value>
───┼──────────────────────────────────────────
 0 │ (name=".azure"
   │ type="Dir"
   │ size=nothing
   │ modified=2020-02-09T05:31:39.950305440Z((B
   │ mdate))
 1 │ (name=".cargo"
   │ type="Dir"
   │ size=nothing
   │ modified=2020-01-06T05:45:30.933303081Z((B
   │ mdate))
───┴──────────────────────────────────────────
❯ ls | last 8 | get type | debug
───┬─────────
 # │ <value>
───┼─────────
 0 │ "Dir"
 1 │ "Dir"
 2 │ "File"
 3 │ "Dir"
 4 │ "File"
 5 │ "Dir"
 6 │ "Dir"
 7 │ "Dir"
───┴─────────
❯ open --raw Cargo.toml | size | debug
(lines=271 words=955 chars=7855 max length=7856)
❯ du src/ | debug
(path="src"(path)
 apparent=705300(bytesize)
 physical=1118208(bytesize)
 directories=[(path="src/utils"(path) apparent=21203(bytesize) physical=24576(bytesize))
  (path="src/data"(path)
   apparent=52860(bytesize)
   physical=86016(bytesize)
   directories=[(path="src/data/config"(path) apparent=2609(bytesize) physical=12288(bytesize))
    (path="src/data/base"(path) apparent=12627(bytesize) physical=16384(bytesize))])
  (path="src/env"(path) apparent=30257(bytesize) physical=36864(bytesize))
  (path="src/plugins"(path) apparent=1358(bytesize) physical=49152(bytesize))
  (path="src/commands"(path)
   apparent=412617(bytesize)
   physical=651264(bytesize)
   directories=[(path="src/commands/classified"(path) apparent=37125(bytesize) physical=49152(bytesize))])
  (path="src/evaluate"(path) apparent=11475(bytesize) physical=24576(bytesize))
  (path="src/format"(path) apparent=15426(bytesize) physical=24576(bytesize))
  (path="src/shell"(path) apparent=81093(bytesize) physical=94208(bytesize))])

```
