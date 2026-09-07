use nu_test_support::prelude::*;
use rstest::rstest;

/// Overflowing `ls -la`-shaped row used to exercise column allocation.
fn overflow_row() -> Value {
    test_value!([{
        name: "grok-continue-115-caching-regression.txt",
        type: "file",
        target: "",
        readonly: false,
        mode: "rw-r--r--",
        num_links: 1,
        inode: 12345678,
        user: "fdncred",
        group: "staff",
        size: 128,
        created: "2026-01-01T00:00:00",
        accessed: "2026-01-01T00:00:00",
        modified: "2026-01-01T00:00:00",
    }])
}

fn inner_widths(table: &str) -> Vec<usize> {
    let row = table.lines().find(|line| {
        (line.contains('│') || line.contains('|'))
            && (line.contains("file") || line.contains("dir") || line.contains("symlink"))
    });
    let Some(row) = row else {
        return Vec::new();
    };
    let delim = if row.contains('│') { '│' } else { '|' };
    row.split(delim)
        .skip(1)
        .filter(|seg| !seg.is_empty())
        .map(|seg| seg.chars().count())
        .collect()
}

fn assert_readable_overflow(rendered: &str, padding_zero: bool) {
    assert!(!rendered.trim().is_empty(), "table rendered empty");
    for needle in [
        "│ s │",
        "│ i │",
        "│ z │",
        "│ e │",
        "| s |",
        "| i |",
        "| z |",
        "| e |",
    ] {
        assert_contains_not(needle, rendered);
    }
    let widths = inner_widths(rendered);
    // With zero padding, a 1-wide index (`0`) is natural. Data columns must not
    // be squeezed to a single character.
    let data_widths: Vec<usize> = if padding_zero && widths.len() >= 2 {
        widths[1..].to_vec()
    } else {
        widths
    };
    assert!(
        !data_widths.contains(&1),
        "1-character data column\n{rendered}"
    );
}

/// Themes change border width, which feeds the wrap/truncate allocator.
#[rstest]
fn trim_stays_readable_for_every_theme(
    #[values(
        "rounded",
        "basic",
        "compact",
        "compact_double",
        "light",
        "thin",
        "with_love",
        "reinforced",
        "heavy",
        "none",
        "psql",
        "markdown",
        "dots",
        "restructured",
        "ascii_rounded",
        "basic_compact",
        "single",
        "double",
        "frameless"
    )]
    theme: &str,
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] header_on_separator: bool,
    #[values(false, true)] expand: bool,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'never'
            $env.config.table.mode = '{theme}'
            $env.config.table.header_on_separator = {header_on_separator}
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table {expand_flag} --width 110
        "
    );
    let rendered: String = test().run_with_data(code, overflow_row())?;
    assert_readable_overflow(&rendered, false);
    Ok(())
}

#[rstest]
fn wrap_and_truncate_differ_for_every_theme_with_header_on_separator(
    #[values(
        "rounded",
        "basic",
        "compact",
        "compact_double",
        "light",
        "thin",
        "with_love",
        "reinforced",
        "heavy",
        "none",
        "psql",
        "markdown",
        "dots",
        "restructured",
        "ascii_rounded",
        "basic_compact",
        "single",
        "double",
        "frameless"
    )]
    theme: &str,
    #[values(false, true)] expand: bool,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let wrapping: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.footer_mode = 'never'
                $env.config.table.mode = '{theme}'
                $env.config.table.header_on_separator = true
                $env.config.table.trim = {{
                    methodology: 'wrapping'
                    wrapping_try_keep_words: true
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width 110
            "
        ),
        overflow_row(),
    )?;
    let truncating: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.footer_mode = 'never'
                $env.config.table.mode = '{theme}'
                $env.config.table.header_on_separator = true
                $env.config.table.trim = {{
                    methodology: 'truncating'
                    wrapping_try_keep_words: true
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width 110
            "
        ),
        overflow_row(),
    )?;
    pretty_assertions::assert_ne!(
        wrapping,
        truncating,
        "theme {theme} expand={expand}: wrapping matched truncating\n{wrapping}"
    );
    Ok(())
}

/// Padding is added on top of the content floor. Zero padding is the
/// 1-character-column regression risk.
#[rstest]
fn trim_stays_readable_for_padding(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] header_on_separator: bool,
    #[values(false, true)] expand: bool,
    #[values(0, 1, 3)] pad: usize,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'never'
            $env.config.table.padding = {{left: {pad}, right: {pad}}}
            $env.config.table.header_on_separator = {header_on_separator}
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table {expand_flag} --width 110
        "
    );
    let rendered: String = test().run_with_data(code, overflow_row())?;
    assert_readable_overflow(&rendered, pad == 0);
    Ok(())
}

#[rstest]
fn trim_stays_readable_for_index_mode(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values("always", "never", "auto")] index_mode: &str,
    #[values(false, true)] expand: bool,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'never'
            $env.config.table.index_mode = '{index_mode}'
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table {expand_flag} --width 110
        "
    );
    let rendered: String = test().run_with_data(code, overflow_row())?;
    assert_readable_overflow(&rendered, false);
    if index_mode == "never" || index_mode == "auto" {
        let first_data = rendered
            .lines()
            .find(|line| line.contains("grok-continue") || line.contains("file"));
        if let Some(line) = first_data {
            assert_contains("grok-continue", line);
        }
    }
    Ok(())
}

#[rstest]
fn missing_value_symbol_survives_overflow_trim(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] expand: bool,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let data = test_value!([{
        name: "grok-continue-115-caching-regression.txt",
        type: "file",
        extra: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    }, {
        name: "short",
        type: "file",
    }]);
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'never'
            $env.config.table.missing_value_symbol = 'NULL'
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table {expand_flag} --width 80
        "
    );
    let rendered: String = test().run_with_data(code, data)?;
    assert_contains("NULL", rendered);
    Ok(())
}

#[rstest]
fn abbreviated_row_count_survives_overflow_trim(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] expand: bool,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let rows = test_value!([
        { name: "file-00-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 0 },
        { name: "file-01-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 1 },
        { name: "file-02-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 2 },
        { name: "file-03-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 3 },
        { name: "file-04-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 4 },
        { name: "file-05-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 5 },
        { name: "file-06-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 6 },
        { name: "file-07-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.txt", type: "file", size: 7 },
    ]);
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'never'
            $env.config.table.abbreviated_row_count = 2
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table {expand_flag} --width 80
        "
    );
    let rendered: String = test().run_with_data(code, rows)?;
    assert_contains("...", rendered);
    Ok(())
}

#[rstest]
fn show_empty_is_independent_of_trim(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] show_empty: bool,
) -> Result {
    let code = format!(
        "
            $env.config.table.trim = {{ methodology: '{methodology}', wrapping_try_keep_words: true, truncating_suffix: '>>' }}
            $env.config.table.show_empty = {show_empty}
            $env.config.use_ansi_coloring = false
            [] | table
        "
    );
    let rendered: String = test().run(code)?;
    if show_empty {
        assert_contains("empty list", rendered);
    } else {
        assert_eq!(rendered.trim(), "");
    }
    Ok(())
}

#[rstest]
#[case::wrapping("wrapping")]
#[case::truncating("truncating")]
fn footer_inheritance_survives_expand_trim(#[case] methodology: &str) -> Result {
    let data = test_value!({
        outer: [
            { a: "left-column-value", b: "right-column-needs-room-xxxxxxxxxxxx" },
            { a: "left-column-value", b: "right-column-needs-room-xxxxxxxxxxxx" },
            { a: "left-column-value", b: "right-column-needs-room-xxxxxxxxxxxx" },
        ]
    });
    let code = format!(
        "
            let data = $in
            $env.config.table.footer_inheritance = true
            $env.config.footer_mode = 'always'
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table --expand --width 80
        "
    );
    let rendered: String = test().run_with_data(code, data)?;
    assert_readable_overflow(&rendered, false);
    Ok(())
}
