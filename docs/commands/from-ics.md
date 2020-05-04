# from ics

Parse text as `.ics` and create table.

Syntax: `from ics`

## Examples

Suppose calendar.txt is a text file that is formatted like a `.ics` (iCal) file:

```shell
> open calendar.txt
BEGIN:VCALENDAR
BEGIN:VEVENT
DTSTART:20171007T200000Z
DTEND:20171007T233000Z
DTSTAMP:20200319T182138Z
SUMMARY:Basketball Game
UID:4l80f6dcovnriq38g57g07btid@google.com
...
```

Pass the output of the `open` command to `from ics` to get a correctly formatted table:

```shell
> open calendar.txt | from ics
───┬────────────────┬──────────────────┬────────────────┬────────────────┬────────────────┬────────────────┬────────────────
 # │ properties     │ events           │ alarms         │ to-Dos         │ journals       │ free-busys     │ timezones
───┼────────────────┼──────────────────┼────────────────┼────────────────┼────────────────┼────────────────┼────────────────
 0 │ [table 0 rows] │ [table 1 row]    │ [table 0 rows] │ [table 0 rows] │ [table 0 rows] │ [table 0 rows] │ [table 0 rows]
───┴────────────────┴──────────────────┴────────────────┴────────────────┴────────────────┴────────────────┴────────────────
```

```shell
> open calendar.txt | from ics | get events | get properties | where name == "SUMMARY"
─────┬─────────┬───────────────────────────────────────┬────────
 #   │ name    │ value                                 │ params
─────┼─────────┼───────────────────────────────────────┼────────
   0 │ SUMMARY │ Basketball Game                       │
```
