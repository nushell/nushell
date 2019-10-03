# env

The `env` command prints to terminal the environment of nushell

This includes 
- cwd     : the path to the current working the directory (`cwd`), 
- home    : the path to the home directory
- config  : the path to the config file for nushell
- history : the path to the nushell command history
- temp 	  : the path to the temp file
- vars    : descriptor variable for the table 

`env` does not take any arguments, and ignores any arguments given.


## Examples -


```shell
/home/username/mynushell/docs/commands(master)> env
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━━━━━━━
 cwd                                    │ home           │ config                                │ history                                    │ temp │ vars 
────────────────────────────────────────┼────────────────┼───────────────────────────────────────┼────────────────────────────────────────────┼──────┼────────────────
 /home/username/mynushell/docs/commands │ /home/username │ /home/username/.config/nu/config.toml │ /home/username/.local/share/nu/history.txt │ /tmp │ [table: 1 row] 
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━━━
```

