[[name, present]; [abc, true], [def, false]]
# | where not present
| get name.0
