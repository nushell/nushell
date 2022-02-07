# ps

This command shows information about system processes.

Syntax: `ps`

## Example

```shell
> ps
━━━━┯━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 #  │ pid   │ name                                                               │ status  │ cpu
────┼───────┼────────────────────────────────────────────────────────────────────┼─────────┼───────────────────
 50 │ 10184 │ firefox.exe                                                        │ Running │ 0.000000000000000
 51 │ 11584 │ WindowsTerminal.exe                                                │ Running │ 0.000000000000000
 52 │ 11052 │ conhost.exe                                                        │ Running │ 0.000000000000000
 53 │  7076 │ nu.exe                                                             │ Running │ 0.000000000000000
   ...
 66 │  3000 │ Code.exe                                                           │ Running │ 0.000000000000000
 67 │  5388 │ conhost.exe                                                        │ Running │ 0.000000000000000
 68 │  6268 │ firefox.exe                                                        │ Running │ 0.000000000000000
 69 │  8972 │ nu_plugin_ps.exe                                                   │ Running │ 58.00986000000000
━━━━┷━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```

Find processes with the highest cpu time
```shell
> ps -l | sort-by cpu_time | last 2
 # │ pid │       name       │ status  │  cpu   │   mem    │ virtual │     cpu_time      │ parent │         exe          │       command
───┼─────┼──────────────────┼─────────┼────────┼──────────┼─────────┼───────────────────┼────────┼──────────────────────┼──────────────────────
 0 │ 396 │ Google Chrome    │ Running │ 0.0000 │ 271.6 MB │  5.8 GB │ 6hr 20min 28sec   │      1 │ /Applications/Google │ /Applications/Google
   │     │                  │         │        │          │         │ 173ms 641us 315ns │        │ Chrome.app/Contents/ │ Chrome.app/Contents/
   │     │                  │         │        │          │         │                   │        │ MacOS/Google         │ MacOS/Google
   │     │                  │         │        │          │         │                   │        │ Chrome               │ Chrome
 1 │ 444 │ Google Chrome He │ Running │ 0.0000 │ 398.9 MB │  5.3 GB │ 10hr 36min 17sec  │    396 │ /Applications/Google │ /Applications/Google
   │     │                  │         │        │          │         │ 304ms 66us 889ns  │        │ Chrome.app/Contents/ │ Chrome.app/Contents/
   │     │                  │         │        │          │         │                   │        │ Frameworks/Google    │ Frameworks/Google
   │     │                  │         │        │          │         │                   │        │ Chrome               │ Chrome
   │     │                  │         │        │          │         │                   │        │ Framework.framework/ │ Framework.framework/
   │     │                  │         │        │          │         │                   │        │ Versions/84.0.4147.1 │ Versions/84.0.4147.1
   │     │                  │         │        │          │         │                   │        │ 25/Helpers/Google    │ 25/Helpers/Google
   │     │                  │         │        │          │         │                   │        │ Chrome Helper        │ Chrome Helper
   │     │                  │         │        │          │         │                   │        │ (GPU).app/Contents/M │ (GPU).app/Contents/M
   │     │                  │         │        │          │         │                   │        │ acOS/Google          │ acOS/Google
   │     │                  │         │        │          │         │                   │        │ Chrome Helper (GPU)  │ Chrome Helper (GPU)
───┴─────┴──────────────────┴─────────┴────────┴──────────┴─────────┴───────────────────┴────────┴──────────────────────┴──────────────────────
```
