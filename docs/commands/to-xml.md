# to xml

Converts table data into XML text.

## Flags

* `-p`, `--pretty` \<integer>: Formats the XML text with the provided indentation setting

## Example

```shell
> open jonathan.xml
━━━━━━━━━━━━━━━━
 rss
────────────────
 [table: 1 row]
━━━━━━━━━━━━━━━━
```

```shell
> cat jonathan.xml
<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel>
                <title>Jonathan Turner</title>
                <link>http://www.jonathanturner.org</link>
                <atom:link href="http://www.jonathanturner.org/feed.xml" rel="self" type="application/rss+xml" />

                        <item>
                                <title>Creating crossplatform Rust terminal apps</title>
        <description>&lt;p&gt;&lt;img src=&quot;/images/pikachu.jpg&quot; alt=&quot;Pikachu animation in Windows&quot; /&gt;&lt;/p&gt;

&lt;p&gt;&lt;em&gt;Look Mom, Pikachu running in Windows CMD!&lt;/em&gt;&lt;/p&gt;

&lt;p&gt;Part of the adventure is not seeing the way ahead and going anyway.&lt;/p&gt;
</description>
<pubDate>Mon, 05 Oct 2015 00:00:00 +0000</pubDate>
<link>http://www.jonathanturner.org/2015/10/off-to-new-adventures.html</link>
<guid isPermaLink="true">http://www.jonathanturner.org/2015/10/off-to-new-adventures.html</guid>
</item>

        </channel>

</rss>
```

```shell
> open jonathan.xml | to xml --pretty 2
<rss version="2.0">
  <channel>
    <title>Jonathan Turner</title>
    <link>http://www.jonathanturner.org</link>
    <link href="http://www.jonathanturner.org/feed.xml" rel="self" type="application/rss+xml">
    </link>
    <item>
      <title>Creating crossplatform Rust terminal apps</title>
      <description>&lt;p&gt;&lt;img src=&quot;/images/pikachu.jpg&quot; alt=&quot;Pikachu animation in Windows&quot; /&gt;&lt;/p&gt;

&lt;p&gt;&lt;em&gt;Look Mom, Pikachu running in Windows CMD!&lt;/em&gt;&lt;/p&gt;

&lt;p&gt;Part of the adventure is not seeing the way ahead and going anyway.&lt;/p&gt;
</description>
<pubDate>Mon, 05 Oct 2015 00:00:00 +0000</pubDate>
<link>http://www.jonathanturner.org/2015/10/off-to-new-adventures.html</link>
<guid isPermaLink="true">http://www.jonathanturner.org/2015/10/off-to-new-adventures.html</guid>
</item>
</channel>
</rss>
```

Due to XML and internal representation, `to xml` is currently limited, it will:

* Only process table data loaded from XML files (e.g. `open file.json | to xml` will fail)
* Drop XML prolog declarations
* Drop namespaces
* Drop comments
