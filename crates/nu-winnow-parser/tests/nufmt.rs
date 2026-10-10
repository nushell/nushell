//! The formatter example must be idempotent and must not change the meaning
//! of a program: formatting then re-parsing yields a structurally identical
//! tree (spans aside) for every file in the corpus, with no comment lost.
#![allow(clippy::unwrap_used, clippy::format_push_string, reason = "tests may panic on failure")]

#[path = "../examples/nufmt/format.rs"]
#[allow(dead_code)]
mod format;

use format::{IndentChar, Note, Options, format, format_with_notes};
use nu_winnow_parser::{parse, pretty};

/// The pretty-printed tree with all spans removed.
fn structure(src: &str) -> String {
    let ast = parse(src).unwrap_or_else(|e| panic!("{}", e.render(src, None)));
    let dump = pretty::dump(&ast);
    let mut out = String::new();
    for line in dump.lines() {
        if line.starts_with("Comments") {
            break;
        }
        let words: Vec<&str> = line
            .split(' ')
            .filter(|w| !(w.contains("..") && w.starts_with(|c: char| c.is_ascii_digit())))
            .filter(|w| !w.starts_with("comments=") && !w.starts_with("terminator="))
            .collect();
        out.push_str(&words.join(" "));
        out.push('\n');
    }
    out
}

/// Format twice with `options`; the second pass must be a no-op and no
/// comment may be lost. Returns the formatted text.
fn check_with(name: &str, src: &str, options: &Options) -> String {
    let once = format(src, options).unwrap_or_else(|e| panic!("{name}: {}", e.render(src, Some(name))));
    let twice = format(&once, options)
        .unwrap_or_else(|e| panic!("{name} (second pass): {}\n---\n{once}", e.render(&once, Some(name))));
    assert_eq!(once, twice, "{name}: formatting is not idempotent");
    let before = parse(src).unwrap().comments.len();
    let after = parse(&once).unwrap().comments.len();
    assert_eq!(before, after, "{name}: comments were lost\n--- formatted:\n{once}");
    once
}

/// With whitespace-only options the tree must not change at all; with the
/// defaults (which may drop redundant parentheses and unquote match
/// patterns) the output must still be idempotent and keep every comment.
fn check(name: &str, src: &str) {
    let once = check_with(name, src, &Options::default().whitespace_only());
    assert_eq!(
        structure(src),
        structure(&once),
        "{name}: formatting changed the program structure\n--- formatted:\n{once}"
    );
    check_with(name, src, &Options::default());
}

#[test]
fn corpus_round_trips() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "nu") {
            let src = std::fs::read_to_string(&path).unwrap();
            check(&path.display().to_string(), &src);
            count += 1;
        }
    }
    assert!(count >= 10);
}

/// The fixtures about comments, which put them in every place a comment can go, including
/// inside text the formatter copies from the source (match patterns, type parameters).
#[test]
fn comment_fixtures_round_trip() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/accept/comments");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "nu") {
            let src = std::fs::read_to_string(&path).unwrap();
            check(&path.display().to_string(), &src);
            count += 1;
        }
    }
    assert!(count >= 10);
}

#[test]
fn normalises_whitespace() {
    let options = Options::default();
    let fmt = |s: &str| format(s, &options).unwrap();
    assert_eq!(fmt("ls|where size > 1kb   |   get name\n"), "ls | where size > 1kb | get name\n");
    assert_eq!(fmt("let   x  =  1  +  2  *  3"), "let x = 1 + 2 * 3\n");
    assert_eq!(fmt("cmd e>| lines"), "cmd e>| lines\n");
    assert_eq!(fmt("cmd o>  out.txt   e> err.txt"), "cmd o> out.txt e> err.txt\n");
    assert_eq!(fmt("[1,2,   3]"), "[1, 2, 3]\n");
    assert_eq!(fmt("{a:1,b:[x y]}"), "{a: 1, b: [x y]}\n");
    assert_eq!(fmt("def f [x:int,--flag(-f)] {$x}"), "def f [x: int, --flag(-f)] { $x }\n");
    assert_eq!(fmt("if $x {1} else {2}"), "if $x { 1 } else { 2 }\n");
    assert_eq!(fmt("def f [] {\nls\n| length\n}"), "def f [] {\n    ls\n    | length\n}\n");
    assert_eq!(fmt("ls\n| each {|x|\n$x\n}"), "ls\n| each {|x| $x }\n");
    assert_eq!(fmt("[\n1\n# c\n2\n]"), "[\n    1\n    # c\n    2\n]\n");
    assert_eq!(fmt("ls # trailing\n\n\n\npwd"), "ls # trailing\n\npwd\n");
    assert_eq!(fmt("each {|x|\n  $x\n}"), "each {|x| $x }\n");
    assert_eq!(fmt("$\"a (1 + 1)\" | print"), "$\"a (1 + 1)\" | print\n");
}

/// A bare word in a `where` condition written without spaces around a
/// comparison (`size>1kb`) is one column name to Nushell, which is never
/// what was meant; the formatter writes the comparison and says so.
#[test]
fn splits_compact_row_conditions() {
    let options = Options::default();
    let fmt = |s: &str| format_with_notes(s, &options).unwrap();
    let (out, notes) = fmt("ls|where size>1kb|get name");
    assert_eq!(out, "ls | where size > 1kb | get name\n");
    assert_eq!(notes, [Note { offset: 9, message: "`size>1kb` written as the comparison `size > 1kb`".into() }]);
    // The rewrite is idempotent: the spaced form parses as a comparison.
    assert_eq!(fmt(&out), (out.clone(), vec![]));

    assert_eq!(fmt("ls | where size>=1kb and name=~Cargo").0, "ls | where size >= 1kb and name =~ Cargo\n");
    assert_eq!(fmt("ls | where not type==dir").0, "ls | where not type == dir\n");
    assert_eq!(fmt("ls | where modified<2024-01-01").0, "ls | where modified < 2024-01-01\n");
    assert_eq!(fmt("ls | where size > 1kb and name!=Cargo").0, "ls | where size > 1kb and name != Cargo\n");
    assert_eq!(fmt("ls | where size>1kb | where name=~Cargo").1.len(), 2);

    // Not a comparison, or not a row condition: left alone, no note.
    for src in [
        "ls | where \"size>1kb\"",
        "ls | where size>>1kb",
        "ls | where size==",
        "ls | where a=1",
        "ls | where name",
        "ls | where name =~ Cargo",
        "ls | where name == a>b",
        "echo size>1kb",
        "ls | where {|row| $row.size > 1kb }",
    ] {
        let (out, notes) = fmt(src);
        assert_eq!(out.trim_end(), src, "{src}");
        assert!(notes.is_empty(), "{src}");
    }
}

fn with(f: impl FnOnce(&mut Options)) -> Options {
    let mut options = Options::default();
    f(&mut options);
    options
}

/// Every option, switched each way, on a small input.
#[test]
fn options_change_layout() {
    let fmt = |s: &str, o: &Options| format(s, o).unwrap();
    let default = Options::default();

    // comment_spacing
    assert_eq!(fmt("ls # c", &default), "ls # c\n");
    assert_eq!(fmt("ls # c", &with(|o| o.comment_spacing = 2)), "ls  # c\n");

    // keep_alignment
    let aligned = "let a     = 1\nlet bbbbb = 2\nsource   x.nu\nls   # c\nmatch $x {\n    \"rtk\"        => 1\n    _            => 2\n}\n";
    assert_eq!(
        fmt(aligned, &default),
        "let a = 1\nlet bbbbb = 2\nsource x.nu\nls # c\nmatch $x {\n    rtk => 1\n    _ => 2\n}\n"
    );
    assert_eq!(
        fmt(aligned, &with(|o| o.keep_alignment = true)),
        "let a     = 1\nlet bbbbb = 2\nsource   x.nu\nls   # c\nmatch $x {\n    rtk          => 1\n    _            => 2\n}\n"
    );

    // trim_trailing_whitespace
    assert_eq!(fmt("ls # c   \n# d  \n", &default), "ls # c\n# d\n");
    assert_eq!(fmt("ls # c   \n# d  \n", &with(|o| o.trim_trailing_whitespace = false)), "ls # c   \n# d  \n");

    // indent_pipelines
    assert_eq!(fmt("ls\n| length", &default), "ls\n| length\n");
    assert_eq!(fmt("ls\n| length", &with(|o| o.indent_pipelines = true)), "ls\n    | length\n");

    // strip_redundant_parens
    let parens = "let x = (ls | length)\nlet y = ((pwd) | where true)\nlet z = ($a + $b)\n(1 + 2)\n(ls)\nif (true) { 1 }\nif ($x | is-empty) { 1 }\ndef f [] { ($in | length) }\nlet w = (bar\n    1\n    2\n)\n";
    assert_eq!(
        fmt(parens, &default),
        "let x = ls | length\nlet y = pwd | where true\nlet z = ($a + $b)\n(1 + 2)\n(ls)\nif true { 1 }\nif ($x | is-empty) { 1 }\ndef f [] { $in | length }\nlet w = (bar\n    1\n    2\n)\n"
    );
    assert_eq!(fmt(parens, &with(|o| o.strip_redundant_parens = false)), parens);
    // ... except those nu needs: around an external command at the start of an
    // assignment's value (`$x = git log` is an error) and around `key:` at the
    // start of a block (`{ echo: 1 }` is a record).
    let needed = "mut x = 1\n$x = (git log)\n$x = ((git log) | lines)\n$x = (ls | length)\nlet c = { (echo: 1) }\nlet d = { (ls) }\n";
    assert_eq!(
        fmt(needed, &default),
        "mut x = 1\n$x = (git log)\n$x = ((git log) | lines)\n$x = ls | length\nlet c = { (echo: 1) }\nlet d = { ls }\n"
    );
    check("parentheses nu needs", needed);

    // expand_def_bodies
    assert_eq!(fmt("def f [] { 1 }\ndef g [] { }", &default), "def f [] { 1 }\ndef g [] { }\n");
    assert_eq!(
        fmt("def f [] { 1 }\ndef g [] { }", &with(|o| o.expand_def_bodies = true)),
        "def f [] {\n    1\n}\ndef g [] { }\n"
    );

    // expand_complex_records
    assert_eq!(fmt("{a: {b: 1}, c: [x y]}", &default), "{\n    a: {b: 1}\n    c: [x y]\n}\n");
    assert_eq!(fmt("{a: 1, c: [x y]}", &default), "{a: 1, c: [x y]}\n");
    assert_eq!(fmt("{a: {b: 1}, c: [x y]}", &with(|o| o.expand_complex_records = false)), "{a: {b: 1}, c: [x y]}\n");

    // compact_simple_closures (and line_length)
    assert_eq!(fmt("each {|x|\n    $x * 2\n}", &default), "each {|x| $x * 2 }\n");
    assert_eq!(fmt("each {|x|\n    let y = $x\n}", &default), "each {|x|\n    let y = $x\n}\n");
    assert_eq!(
        fmt("each {|x|\n    $x * 2\n}", &with(|o| o.compact_simple_closures = false)),
        "each {|x|\n    $x * 2\n}\n"
    );
    let long = format!("each {{|x|\n    $x + {}\n}}", "1".repeat(80));
    assert_eq!(fmt(&long, &default), format!("each {{|x|\n    $x + {}\n}}\n", "1".repeat(80)));
    assert_eq!(fmt(&long, &with(|o| o.line_length = 200)), format!("each {{|x| $x + {} }}\n", "1".repeat(80)));

    // unquote_match_patterns
    let arms = "match $x {\n    \"allow\" => 1\n    \"true\" => 2\n    \"a-b\" => 3\n    \"_\" => 4\n}\n";
    assert_eq!(
        fmt(arms, &default),
        "match $x {\n    allow => 1\n    \"true\" => 2\n    \"a-b\" => 3\n    \"_\" => 4\n}\n"
    );
    assert_eq!(fmt(arms, &with(|o| o.unquote_match_patterns = false)), arms);
    // A string nu would read as a number once unquoted keeps its quotes
    // (bare, `Infinity`, `NAN`, `_1` and `i_n_f` are numbers; `e5` is not).
    let numbers = "match $x {\n    \"Infinity\" => 1\n    \"NAN\" => 2\n    \"_1\" => 3\n    \"i_n_f\" => 4\n    \"e5\" => 5\n}\n";
    assert_eq!(fmt(numbers, &default), numbers.replace("\"e5\"", "e5"));

    // margin and declaration grouping
    let items = "echo one\necho two\n\n\necho three\n";
    assert_eq!(fmt(items, &default), "echo one\necho two\n\necho three\n");
    assert_eq!(fmt(items, &with(|o| o.margin = Some(1))), "echo one\n\necho two\n\necho three\n");
    assert_eq!(fmt(items, &with(|o| o.margin = Some(0))), "echo one\necho two\necho three\n");
    let decls = "let a = 1\n\nlet b = 2\nconst c = 3\nconst d = [\n    1\n    2\n]\nuse x.nu\n\nuse y.nu\n";
    assert_eq!(
        fmt(decls, &default),
        "let a = 1\nlet b = 2\n\nconst c = 3\n\nconst d = [\n    1\n    2\n]\nuse x.nu\nuse y.nu\n"
    );
    assert_eq!(
        fmt("def f [] {\n    let a = 1\n\n    let b = 2\n}", &default),
        "def f [] {\n    let a = 1\n\n    let b = 2\n}\n"
    );

    // indent and indent_char
    assert_eq!(fmt("def f [] {\n1\n}", &with(|o| o.indent = 2)), "def f [] {\n  1\n}\n");
    assert_eq!(fmt("def f [] {\n1\n}", &with(|o| o.indent_char = IndentChar::Tab)), "def f [] {\n\t1\n}\n");
}

/// Layout that follows the source: comments in every position, items grouped
/// per line, tables, and arguments split over lines.
#[test]
fn preserves_source_layout() {
    let fmt = |s: &str| format(s, &Options::default()).unwrap();
    let same = |s: &str| assert_eq!(fmt(s), s, "{s}");
    same("match $x {\n    a => 1\n    # between arms\n    b if $y => 2\n}\n");
    same("let x = [\n    1 # one\n]\n");
    same("let x = [ # opener\n    1\n    2\n]\n");
    same("let x = { # opener\n    a: 1\n}\n");
    same("def f [] { # opener\n    1\n}\n");
    same("# first\n\n# second\nls\n\n# third\n\nls\n");
    same("let x = [\n    \"--flag\" value\n    \"--other\" 1\n]\n");
    same("const t = [\n    [a, b];\n    [1, 2]\n    [3, 4]\n]\n");
    same("let x = (bar\n    1\n    2\n)\n");
    same("{|k v| $k }\n");
    same("def f [a b] { $a }\n");
    assert_eq!(fmt("[\n    1\n]"), "[1]\n");
    assert_eq!(fmt("$\"(# not a comment)\""), "$\"(# not a comment)\"\n");
    assert_eq!(fmt("{a: 1}\n"), "{a: 1}\n");
    assert_eq!(fmt("use std [a,  b]"), "use std [a, b]\n");
    assert_eq!(fmt("error  make {msg: 1}"), "error make {msg: 1}\n");
    assert_eq!(fmt("match $x {\n    {type:  \"user\"} => 1\n}"), "match $x {\n    {type: \"user\"} => 1\n}\n");
    // A `use` list with a comment keeps its lines: joined, the comment would hide the rest.
    same("use m [\n    a # first\n    b\n]\n");
    // Spaces inside a string with an escaped quote, or a raw string, are its text.
    let strings = "match $x {\n    \"a\\\"  b\" => 1\n    r#'it's  here'# => 2\n}\n";
    same(strings);
    check("spaces inside strings", strings);
    // Statements of a one-line subexpression: one `;` between them.
    same("print (1; 2)\n");
    assert_eq!(fmt("(1;2;)"), "(1; 2;)\n");
}

/// Comments inside a signature or a closure's parameters are written once and
/// keep their lines.
#[test]
fn keeps_comments_in_parameters() {
    let fmt = |s: &str| format(s, &Options::default()).unwrap();
    for src in [
        "def f [\n    # c\n] { }\n",
        "extern foo [\n    # TODO\n]\n",
        "def f [\n    # c\n]: nothing -> int { 1 }\n",
        "let c = {|x: record<\n    a: int # field a\n>|\n    $x\n}\n",
        "{|x, # the x value\n  y # the y value\n|\n    $x + $y\n}\n",
    ] {
        assert_eq!(fmt(src), src, "{src}");
        check(src, src);
    }
    // A comment on the `[` line of a list without parameters gets its own line.
    assert_eq!(fmt("def f [ # opener\n] { 1 }\n"), "def f [\n    # opener\n] { 1 }\n");
    // A comment in a closure parameter's type or default is not written again in the body.
    for src in ["let c = {|x: record<\n    a: int # field a\n>| $x }\n", "{|x = [\n    1 # one\n]| $x }\n"] {
        check(src, src);
    }
}

/// A `--` after a keyword is text nu drops, but it decides how the next word
/// is read: `return -- -1` returns -1, `return -1` is a flag `return` lacks.
#[test]
fn keeps_end_of_options_after_keywords() {
    let fmt = |s: &str| format(s, &Options::default()).unwrap();
    for src in [
        "def f [] { return -- -1 }\n",
        "return -- --help\n",
        "if -- -1 < 0 { 1 }\n",
        "while -- true { break }\n",
        "match -- -1 {\n    _ => 2\n}\n",
        "match 1 -- {\n    _ => 2\n}\n",
        "for -- x in [1] { }\n",
        "for x in [1] -- { }\n",
        "try -- { }\n",
        "try { } -- catch { }\n",
        "try { } --\n",
        "module x { } --\n",
        "let -- x\n",
        "def --env -- f [] { }\n",
        "extern foo -- []\n",
        "use -- std\n",
    ] {
        assert_eq!(fmt(src), src, "{src}");
        check(src, src);
    }
    // A `--` that nu drops inside a list (a redirection's target) is not one of
    // them, and never becomes an item.
    let out = fmt("[a o> -- b]\n");
    assert!(!out.contains("a -- b"), "{out}");
}

/// `if(true){1}else{2}` is one word to Nushell; it is written as an `if`.
#[test]
fn repairs_packed_if() {
    let (out, notes) = format_with_notes("def f [] {\n    if(true){1}else{2}\n}", &Options::default()).unwrap();
    assert_eq!(out, "def f [] {\n    if true { 1 } else { 2 }\n}\n");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].offset, 15);
    let (out, notes) = format_with_notes("iffy(1)", &Options::default()).unwrap();
    assert_eq!((out.as_str(), notes.len()), ("iffy(1)\n", 0));
}
