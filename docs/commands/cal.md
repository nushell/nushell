---
title: cal
layout: command
version: 0.59.0
---

Display a calendar.

## Signature

```> cal --year --quarter --month --full-year --week-start --month-names```

## Parameters

 -  `--year`: Display the year column
 -  `--quarter`: Display the quarter column
 -  `--month`: Display the month column
 -  `--full-year {int}`: Display a year-long calendar for the specified year
 -  `--week-start {string}`: Display the calendar with the specified day as the first day of the week
 -  `--month-names`: Display the month names instead of integers

## Examples

This month's calendar
```shell
> cal
```

The calendar for all of 2012
```shell
> cal --full-year 2012
```

This month's calendar with the week starting on monday
```shell
> cal --week-start monday
```
