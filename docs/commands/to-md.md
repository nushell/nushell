# to md

Convert table into simple Markdown.

## Flags

* `-p`, `--pretty`: Formats the Markdown table to vertically align items

## Example

```shell
> ls | to md
|name|type|size|modified|
|-|-|-|-|
|CODE_OF_CONDUCT.md|File|3.4 KB|2 months ago|
|CONTRIBUTING.md|File|1.4 KB|1 month ago|
|Cargo.lock|File|144.4 KB|2 days ago|
|Cargo.toml|File|6.0 KB|2 days ago|
|LICENSE|File|1.1 KB|2 months ago|
|Makefile.toml|File|449 B|2 months ago|
|README.build.txt|File|192 B|2 months ago|
|README.md|File|15.9 KB|1 month ago|
|TODO.md|File|0 B|2 months ago|
|crates|Dir|896 B|2 days ago|
|debian|Dir|352 B|2 months ago|
|docker|Dir|288 B|1 month ago|
|docs|Dir|256 B|1 month ago|
|features.toml|File|632 B|2 months ago|
|images|Dir|160 B|2 months ago|
|pkg_mgrs|Dir|96 B|1 month ago|
|rustfmt.toml|File|16 B|9 months ago|
|samples|Dir|96 B|1 month ago|
|src|Dir|128 B|2 days ago|
|target|Dir|160 B|1 month ago|
|tests|Dir|192 B|2 months ago|
|wix|Dir|128 B|23 hours ago|
```

If we provide the `-p` flag, we can obtain a formatted version of the Markdown table

```shell
> ls | to md  -p
|name              |type|size    |modified    |
|------------------|----|--------|------------|
|CODE_OF_CONDUCT.md|File|3.4 KB  |2 months ago|
|CONTRIBUTING.md   |File|1.4 KB  |1 month ago |
|Cargo.lock        |File|144.4 KB|2 days ago  |
|Cargo.toml        |File|6.0 KB  |2 days ago  |
|LICENSE           |File|1.1 KB  |2 months ago|
|Makefile.toml     |File|449 B   |2 months ago|
|README.build.txt  |File|192 B   |2 months ago|
|README.md         |File|15.9 KB |1 month ago |
|TODO.md           |File|0 B     |2 months ago|
|crates            |Dir |896 B   |2 days ago  |
|debian            |Dir |352 B   |2 months ago|
|docker            |Dir |288 B   |1 month ago |
|docs              |Dir |256 B   |1 month ago |
|features.toml     |File|632 B   |2 months ago|
|images            |Dir |160 B   |2 months ago|
|pkg_mgrs          |Dir |96 B    |1 month ago |
|rustfmt.toml      |File|16 B    |9 months ago|
|samples           |Dir |96 B    |1 month ago |
|src               |Dir |128 B   |2 days ago  |
|target            |Dir |160 B   |1 month ago |
|tests             |Dir |192 B   |2 months ago|
|wix               |Dir |128 B   |23 hours ago|
```
