---
title: update cells
layout: command
version: 0.59.1
---

Update the table cells.

## Signature

```> update cells (block) --columns```

## Parameters

 -  `block`: the block to run an update for each cell
 -  `--columns {table}`: list of columns to update

## Examples

Update the zero value cells to empty strings.
```shell
> [
    ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
    [          37,            0,            0,            0,           37,            0,            0]
] | update cells {|value|
      if $value == 0 {
        ""
      } else {
        $value
      }
}
```

Update the zero value cells to empty strings in 2 last columns.
```shell
> [
    ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
    [          37,            0,            0,            0,           37,            0,            0]
] | update cells -c ["2021-11-18", "2021-11-17"] {|value|
        if $value == 0 {
          ""
        } else {
          $value
        }
}
```
