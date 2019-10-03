# to-yaml

Converts table data into yaml text.

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
> shells | to-yaml
---
- " ": X
  name: filesystem
  path: /home/shaurya
- " ": " "
  name: filesystem
  path: /home/shaurya/Pictures
- " ": " "
  name: filesystem
  path: /home/shaurya/Desktop
```
