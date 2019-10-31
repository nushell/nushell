# from-json
Converts json data into table. Use this when nushell cannot dertermine the input file extension.

## Example
Let's say we have the following sample menu.json file:
```shell
> open menu.json
{
"menu": {
	"id": "file",
	"value": "File",
	"popup": {
		"menuitem": [
			"New",
			"Open",
			"Close"
		]
	}
}
}
```

The "menu.json" file is actually a .json file, but the file extension isn't .json. That's okay, we can use the `from-json` command :


```shell
> open menu.json | from-json
━━━━━━━━━━━━━━━━
 menu       
────────────────
 [table: 1 row]
━━━━━━━━━━━━━━━━
```
