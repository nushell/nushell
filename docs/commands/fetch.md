# fetch

This command loads from a URL into a cell, convert it to table if possible (avoid by appending `--raw` flag)

## Examples

```shell
> fetch http://headers.jsontest.com
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━
 X-Cloud-Trace-Context                                 │ Accept │ Host                 │ Content-Length │ user-agent
───────────────────────────────────────────────────────┼────────┼──────────────────────┼────────────────┼─────────────────────────
 aeee1a8abf08820f6fe19d114dc3bb87/16772233176633589121 │ */*    │ headers.jsontest.com │ 0              │ curl/7.54.0 isahc/0.7.1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━
> fetch http://headers.jsontest.com --raw
{
   "X-Cloud-Trace-Context": "aeee1a8abf08820f6fe19d114dc3bb87/16772233176633589121",
   "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3",
   "Upgrade-Insecure-Requests": "1",
   "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/77.0.3865.90 Safari/537.36",
   "Host": "headers.jsontest.com",
   "Accept-Language": "en-GB,en-US;q=0.9,en;q=0.8"
}
```

```shell
> fetch https://www.jonathanturner.org/feed.xml
━━━━━━━━━━━━━━━━
 rss
────────────────
 [table: 1 row]
━━━━━━━━━━━━━━━━
```