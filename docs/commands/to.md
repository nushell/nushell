# to

Converts table data into a string or binary. The target format is specified as a subcommand, like `to csv` or `to json`.

## Available Subcommands

* to bson
* [to csv](to-csv.md)
* to html
* [to json](to-json.md)
* [to md](to-md.md)
* to sqlite
* [to toml](to-toml.md)
* [to tsv](to-tsv.md)
* [to url](to-url.md)
* [to xml](to-xml.md)
* [to yaml](to-yaml.md)

*Subcommands without links are currently missing their documentation.*

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
```

```shell
> shells | to csv
 ,name,path
X,filesystem,/home/shaurya
 ,filesystem,/home/shaurya/Pictures
 ,filesystem,/home/shaurya/Desktop
```

```shell
> open sample.url
━━━━━━━━━━┯━━━━━━━━┯━━━━━━┯━━━━━━━━
 bread    │ cheese │ meat │ fat
──────────┼────────┼──────┼────────
 baguette │ comté  │ ham  │ butter
━━━━━━━━━━┷━━━━━━━━┷━━━━━━┷━━━━━━━━
```

```shell
> open sample.url  | to url
bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter
```
