# to-csv

Converts table data into csv text.

## Example

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path 
───┼───┼────────────┼────────────────────────
 0 │ X │ filesystem │ /home/shaurya 
 1 │   │ filesystem │ /home/shaurya/Pictures 
 2 │   │ filesystem │ /home/shaurya/Desktop 
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━
> shells | to-csv
 ,name,path
X,filesystem,/home/shaurya
 ,filesystem,/home/shaurya/Pictures
 ,filesystem,/home/shaurya/Desktop
```
