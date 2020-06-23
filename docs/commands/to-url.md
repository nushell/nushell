# to url

Converts table data into [url-encoded text](https://url.spec.whatwg.org/#application/x-www-form-urlencoded).

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
> shells | to url
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ value
───┼───────────────────────────────────────────────────────
 0 │ +=X&name=filesystem&path=%2Fhome%2Fshaurya
 1 │ +=+&name=filesystem&path=%2Fhome%2Fshaurya%2FPictures
 2 │ +=+&name=filesystem&path=%2Fhome%2Fshaurya%2FDesktop
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
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
