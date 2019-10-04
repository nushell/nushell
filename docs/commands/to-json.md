# to-json

Converts table data into json text.

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
> shells | to-json
[{" ":"X","name":"filesystem","path":"/home/shaurya"},{" ":" ","name":"filesystem","path":"/home/shaurya/Pictures"},{" ":" ","name":"filesystem","path":"/home/shaurya/Desktop"}]
```

```shell
> open sgml_description.json 
━━━━━━━━━━━━━━━━
 glossary 
────────────────
 [table: 1 row] 
━━━━━━━━━━━━━━━━
> open sgml_description.json | to-json
{"glossary":{"title":"example glossary","GlossDiv":{"title":"S","GlossList":{"GlossEntry":{"ID":"SGML","SortAs":"SGML","GlossTerm":"Standard Generalized Markup Language","Acronym":"SGML","Abbrev":"ISO 8879:1986","Height":10,"GlossDef":{"para":"A meta-markup language, used to create markup languages such as DocBook.","GlossSeeAlso":["GML","XML"]},"Sections":[101,102],"GlossSee":"markup"}}}}}
```
We can also convert formats !
```shell
> open jonathan.xml
━━━━━━━━━━━━━━━━
 rss 
────────────────
 [table: 1 row] 
━━━━━━━━━━━━━━━━
> open jonathan.xml | to-json
{"rss":[{"channel":[{"title":["Jonathan Turner"]},{"link":["http://www.jonathanturner.org"]},{"link":[]},{"item":[{"title":["Creating crossplatform Rust terminal apps"]},{"description":["<p><img src=\"/images/pikachu.jpg\" alt=\"Pikachu animation in Windows\" /></p>\n\n<p><em>Look Mom, Pikachu running in Windows CMD!</em></p>\n\n<p>Part of the adventure is not seeing the way ahead and going anyway.</p>\n"]},{"pubDate":["Mon, 05 Oct 2015 00:00:00 +0000"]},{"link":["http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"]},{"guid":["http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"]}]}]}]}
```
