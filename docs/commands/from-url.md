# from url

Parse [url-encoded string](https://url.spec.whatwg.org/#application/x-www-form-urlencoded) as a table.

## Example

```shell
> echo 'bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter' | from url
────────┬──────────
 bread  │ baguette
 cheese │ comté
 meat   │ ham
 fat    │ butter
────────┴──────────
```
