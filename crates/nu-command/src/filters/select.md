# Select implementation strategies

Compiled by fdncred.


## POSIX, `grep`

Original at least 1 char long
```bash
> # benchmark INCLUDING DOWNLOAD: 1sec 253ms 91µs 511ns
> curl -sL "https://www.gutenberg.org/files/11/11-0.txt" | tr '[:upper:]' '[:lower:]' | grep -oE "[a-z\']{1,}" | sort | uniq -c | sort -nr | head -n 10
   1839 the
    942 and
    811 to
    695 a
    638 of
    610 it
    553 she
    546 i
    486 you
    462 said
```

Original at least 2 chars long

```bash
curl -sL "https://www.gutenberg.org/files/11/11-0.txt" | tr '[:upper:]' '[:lower:]' | grep -oE "[a-z\']{2,}" | sort | uniq -c | sort -nr | head -n 10
   1839 the
    942 and
    811 to
    638 of
    610 it
    553 she
    486 you
    462 said
    435 in
    403 alice
```


### Nushell, regex

Regex means, replace everything that is not A-Z or a-z or ' with a space

```Nushell
> # benchmark: 1sec 775ms 471µs 600ns
> $contents | str replace "[^A-Za-z\']" " " -a | split row ' ' | where ($it | str length) > 1 | uniq -i -c | sort-by count --reverse | first 10
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1839 │
│ 1 │ and   │   942 │
│ 2 │ to    │   811 │
│ 3 │ of    │   638 │
│ 4 │ it    │   610 │
│ 5 │ she   │   553 │
│ 6 │ you   │   486 │
│ 7 │ said  │   462 │
│ 8 │ in    │   435 │
│ 9 │ alice │   403 │
╰───┴───────┴───────╯
```

```Nushell
> # benchmark: 1sec 518ms 701µs 200ns
> $alice |str replace "[^A-Za-z\']" " " -a | split row ' ' | uniq -i -c | sort-by count --reverse | first 10
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1839 │
│ 1 │ and   │   942 │
│ 2 │ to    │   811 │
│ 3 │ a     │   695 │
│ 4 │ of    │   638 │
│ 5 │ it    │   610 │
│ 6 │ she   │   553 │
│ 7 │ i     │   546 │
│ 8 │ you   │   486 │
│ 9 │ said  │   462 │
├───┼───────┼───────┤
│ # │ value │ count │
╰───┴───────┴───────╯
```


### Nushell, `unicode_words()`

```Nushell
> # benchmark: 4sec 965ms 285µs 800ns
> $alice | str downcase | split words | sort | uniq -c | sort-by count | reverse | first 10
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1839 │
│ 1 │ and   │   941 │
│ 2 │ to    │   811 │
│ 3 │ a     │   695 │
│ 4 │ of    │   638 │
│ 5 │ it    │   542 │
│ 6 │ she   │   538 │
│ 7 │ said  │   460 │
│ 8 │ in    │   434 │
│ 9 │ you   │   426 │
├───┼───────┼───────┤
│ # │ value │ count │
╰───┴───────┴───────╯
```

### Rust, `trim_to_words`

```Nushell
benchmark: 5sec 992ms 76µs 200ns
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1829 │
│ 1 │ and   │   918 │
│ 2 │ to    │   801 │
│ 3 │ a     │   689 │
│ 4 │ of    │   632 │
│ 5 │ she   │   537 │
│ 6 │ it    │   493 │
│ 7 │ said  │   457 │
│ 8 │ in    │   430 │
│ 9 │ you   │   413 │
├───┼───────┼───────┤
│ # │ value │ count │
╰───┴───────┴───────╯
```

```Rust
fn trim_to_words(content: String) -> std::vec::Vec<std::string::String> {
    let content: Vec<String> = content
        .to_lowercase()
        .replace(&['-'][..], " ")
        //should 's be replaced?
        .replace("'s", "")
        .replace(
            &[
                '(', ')', ',', '\"', '.', ';', ':', '=', '[', ']', '{', '}', '-', '_', '/', '\'',
                '’', '?', '!', '“', '‘',
            ][..],
            "",
        )
        .split_whitespace()
        .map(String::from)
        .collect::<Vec<String>>();
    content
}
```


### Rust, `split_whitespace()` from std

```Nushell
> # benchmark: 9sec 379ms 790µs 900ns
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1683 │
│ 1 │ and   │   783 │
│ 2 │ to    │   778 │
│ 3 │ a     │   667 │
│ 4 │ of    │   605 │
│ 5 │ she   │   485 │
│ 6 │ said  │   416 │
│ 7 │ in    │   406 │
│ 8 │ it    │   357 │
│ 9 │ was   │   329 │
├───┼───────┼───────┤
│ # │ value │ count │
╰───┴───────┴───────╯
```


### Rust, regex

There are some options here with this regex.

- `[^A-Za-z\']` do not match uppercase or lowercase letters or
  apostrophes.
- `[^[:alpha:]\']`: do not match any uppercase or lowercase letters or
  apostrophes.
- `[^\p{L}\']`: do not match any Unicode uppercase or lowercase letters
  or apostrophes.

Let's go with the Unicode one in hopes that it works on more than just
ASCII characters.

```Nushell
> # benchmark: 1sec 481ms 604µs 700ns
> $alice | str downcase | split words | uniq -c | sort-by count --reverse | first 10
╭───┬───────┬───────╮
│ # │ value │ count │
├───┼───────┼───────┤
│ 0 │ the   │  1839 │
│ 1 │ and   │   942 │
│ 2 │ to    │   811 │
│ 3 │ a     │   695 │
│ 4 │ of    │   638 │
│ 5 │ it    │   610 │
│ 6 │ she   │   553 │
│ 7 │ i     │   546 │
│ 8 │ you   │   486 │
│ 9 │ said  │   462 │
├───┼───────┼───────┤
│ # │ value │ count │
╰───┴───────┴───────╯
```
