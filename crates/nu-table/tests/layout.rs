//! Layout sweeps: real-world table shapes drawn at every width from narrow to
//! wide under every trim, header, index and padding setting, checked against
//! the column width rules instead of exact snapshots. One sweep draws plain
//! tables with `NuTable`, the other draws `table --expand` output with
//! `ExpandedTable`.
//!
//! Every draw follows these rules:
//!
//! - No line is wider than the width. Every line of a bordered table is
//!   equally wide, its column borders line up with the top border, and nested
//!   tables keep their borders.
//! - A table that fits keeps every column at its natural width. An expanded
//!   table that already fills most of the width stretches to all of it; the
//!   index column (a record's keys) keeps its width.
//! - Otherwise columns shrink in tiers: the plain columns, then the
//!   width-priority columns, then the index column. A tier shrinks only once
//!   every tier before it is at its floors; within a tier the widest shrink first.
//! - No column shrinks below its floor: its natural width, at most 10
//!   characters plus padding (more for a long truncation suffix), at least the
//!   header when the header sits on the border. Only when not even the first
//!   data column fits at its floor is it squeezed, to 4 characters.
//! - Columns are hidden from the right, behind a `...` column, only when the
//!   floors of the visible columns plus the next one do not fit, to give a
//!   visible width-priority column a third of the width, or to give a visible
//!   column of nested tables the width to draw them.
//! - A header drawn on the border is drawn in full, and a table with an
//!   index column shows at least one data column.
//!
//! And across the widths of one table:
//!
//! - The same input draws the same output.
//! - A wider table never shows fewer columns, unless it draws more nested
//!   tables in their place.
//! - Marking a column as a width priority never hides it or makes it narrower.
//! - Every table renders once its borders, its index column at its floor and
//!   one squeezed column (plus `...` when more remain) fit.
//!
//! A column holding a drawn nested table keeps the width of that table, so the
//! shrinking rules skip it. Violations are collected over the whole sweep and
//! reported together.

mod common;

use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use common::{TestCase, bordered_width, create_table, split_cells};
use nu_color_config::StyleComputer;
use nu_engine::column::get_columns;
use nu_protocol::{
    Config, Record, Signals, Span, TableIndent, TableIndexMode, TableMode, TrimStrategy, Value,
    engine::{EngineState, Stack},
};
use nu_table::{ExpandedTable, TableOpts, TableTheme, string_width};
use tabled::grid::{records::vec_records::Text, util::string::get_char_width};

/// Content width a column keeps before columns get hidden.
const FLOOR: usize = 10;
/// Content width the first data column is squeezed to when nothing else fits.
const SQUEEZED: usize = 4;
/// Content width of the trailing `...` column.
const TRAIL: usize = 3;
/// Widths drawn twice to check that drawing is deterministic.
const DETERMINISM_WIDTHS: [usize; 3] = [30, 60, 120];

/// The widths a sweep draws at from `narrowest` on: every width up to 80,
/// where most of the shrinking and hiding happens, then every second up to
/// 120, then every fourth.
fn sweep_widths(narrowest: usize) -> impl Iterator<Item = usize> {
    (narrowest..=80)
        .chain((82..=120).step_by(2))
        .chain((124..=200).step_by(4))
}

/// Runs `job` for every shape and setup on all cores, merging the reports.
fn run_sweeps<S: Sync>(shapes: &[S], setups: &[Setup], job: impl Fn(&S, Setup) -> Report + Sync) {
    let jobs: Vec<(&S, Setup)> = shapes
        .iter()
        .flat_map(|shape| setups.iter().map(move |&setup| (shape, setup)))
        .collect();
    let next = AtomicUsize::new(0);
    let threads = thread::available_parallelism().map_or(4, |n| n.get());

    let mut report = Report::default();
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut report = Report::default();
                    while let Some(&(shape, setup)) = jobs.get(next.fetch_add(1, Ordering::Relaxed))
                    {
                        report.merge(job(shape, setup));
                    }
                    report
                })
            })
            .collect();
        for worker in workers {
            report.merge(worker.join().expect("sweep thread"));
        }
    });

    report.assert_clean();
}

const WORDS: &str =
    "the quick brown fox jumps over the lazy dog and keeps running across the field";
const LONG_NAME: &str =
    "very_very_very_long_filename_that_should_get_priority_and_avoid_wrapping.txt";

fn path_entries() -> Vec<&'static str> {
    vec![
        "/Users/fdncred/.local/bin",
        "/usr/local/bin",
        "/opt/homebrew/opt/python/libexec/bin",
        "/Users/fdncred/.cargo/bin",
        "/opt/homebrew/sbin",
        "/opt/homebrew/bin",
        "/opt/homebrew/opt/uutils-coreutils/libexec/uubin",
        "/System/Cryptexes/App/usr/bin",
        "/usr/bin",
        "/bin",
    ]
}

#[derive(Clone, Copy)]
enum Trim {
    Wrap,
    WrapAnyChar,
    Truncate,
    TruncateLongSuffix,
}

impl Trim {
    fn suffix(self) -> Option<&'static str> {
        match self {
            Trim::Truncate => Some("…"),
            Trim::TruncateLongSuffix => Some("[...]"),
            Trim::Wrap | Trim::WrapAnyChar => None,
        }
    }
}

/// One table setting combination.
#[derive(Clone, Copy)]
struct Setup {
    trim: Trim,
    header_on_border: bool,
    index: bool,
    pad: usize,
    bordered: bool,
}

impl Setup {
    fn label(&self) -> String {
        let trim = match self.trim {
            Trim::Wrap => "wrap",
            Trim::WrapAnyChar => "wrap-chars",
            Trim::Truncate => "truncate",
            Trim::TruncateLongSuffix => "truncate-long-suffix",
        };
        format!(
            "{trim},hos={},index={},pad={},theme={}",
            self.header_on_border,
            self.index,
            self.pad,
            if self.bordered { "rounded" } else { "none" }
        )
    }

    fn strategy(&self) -> TrimStrategy {
        match self.trim {
            Trim::Wrap => TrimStrategy::wrap(true),
            Trim::WrapAnyChar => TrimStrategy::wrap(false),
            Trim::Truncate | Trim::TruncateLongSuffix => {
                TrimStrategy::truncate(self.trim.suffix().map(String::from))
            }
        }
    }

    fn indent(&self) -> TableIndent {
        TableIndent::new(self.pad, self.pad)
    }

    /// A content width, widened to hold a truncation suffix and one more character.
    fn content(&self, width: usize) -> usize {
        let suffix = self.trim.suffix().map_or(0, |s| string_width(s) + 1);
        width.max(suffix)
    }

    /// The floor of a column `natural` wide (padding included), with a
    /// `header` that wide on the border.
    fn floor(&self, natural: usize, header: Option<usize>) -> usize {
        let pad = 2 * self.pad;
        let header = header.map_or(0, |header| header + pad);
        (self.content(FLOOR) + pad).max(header).min(natural)
    }

    /// The width the first data column is squeezed to when nothing else fits.
    fn squeezed(&self, natural: usize) -> usize {
        (self.content(SQUEEZED) + 2 * self.pad).min(natural)
    }

    fn all(themes: &[bool]) -> Vec<Setup> {
        let mut out = vec![];
        for trim in [
            Trim::Wrap,
            Trim::WrapAnyChar,
            Trim::Truncate,
            Trim::TruncateLongSuffix,
        ] {
            for header_on_border in [false, true] {
                for index in [false, true] {
                    for pad in [1, 0] {
                        for &bordered in themes {
                            out.push(Setup {
                                trim,
                                header_on_border,
                                index,
                                pad,
                                bordered,
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

/// What the rules need to know about a table's columns.
struct Spec {
    /// Natural column widths, padding included.
    natural: Vec<usize>,
    /// Whether the first column is the index column (a record's keys).
    index: bool,
    priorities: Vec<usize>,
    /// Columns holding nested lists or records.
    nested: Vec<usize>,
    /// Column headers, empty for a table without a header row.
    headers: Vec<String>,
    /// An expanded table stretches to the full width once it fills 80% of it.
    stretches: bool,
    setup: Setup,
}

impl Spec {
    /// The floor of every column, with the header on the border or not.
    fn floors(&self, on_border: bool) -> Vec<usize> {
        self.natural
            .iter()
            .enumerate()
            .map(|(col, &natural)| {
                let header = self.headers.get(col).filter(|_| on_border);
                self.setup.floor(natural, header.map(|h| string_width(h)))
            })
            .collect()
    }

    fn first_data(&self) -> usize {
        usize::from(self.index && self.natural.len() > 1)
    }

    /// The shrink tier of a column: plain, width priority, index.
    fn tier(&self, col: usize) -> usize {
        if self.index && col == 0 {
            2
        } else {
            usize::from(self.priorities.contains(&col))
        }
    }

    /// The width at which the table must render: its borders, the index
    /// column at its floor, the first data column squeezed, and `...` when
    /// more columns remain.
    fn render_width(&self) -> usize {
        let first = self.first_data();
        let mut widths = vec![];
        if first == 1 {
            widths.push(self.setup.floor(self.natural[0], None));
        }
        widths.push(self.setup.squeezed(self.natural[first]));
        if self.natural.len() > first + 1 {
            widths.push(TRAIL + 2 * self.setup.pad);
        }
        bordered_width(&widths)
    }
}

/// A rule broken by one draw.
struct Violation {
    check: &'static str,
    shape: &'static str,
    case: String,
    detail: String,
}

#[derive(Default)]
struct Report {
    violations: Vec<Violation>,
    draws: usize,
}

impl Report {
    fn push(&mut self, check: &'static str, case: &Case, detail: String) {
        self.violations.push(Violation {
            check,
            shape: case.shape,
            case: case.label(),
            detail,
        });
    }

    fn merge(&mut self, other: Report) {
        self.violations.extend(other.violations);
        self.draws += other.draws;
    }

    fn assert_clean(self) {
        if self.violations.is_empty() {
            return;
        }

        let mut by_check = BTreeMap::<&str, usize>::new();
        let mut by_shape = BTreeMap::<&str, usize>::new();
        // One example per (check, shape) pair keeps the report short but complete.
        let mut examples = BTreeMap::<(&str, &str), String>::new();
        for violation in &self.violations {
            *by_check.entry(violation.check).or_default() += 1;
            *by_shape.entry(violation.shape).or_default() += 1;
            examples
                .entry((violation.check, violation.shape))
                .or_insert_with(|| {
                    format!(
                        "[{}] {}: {}",
                        violation.check, violation.case, violation.detail
                    )
                });
        }
        panic!(
            "{} of {} draws broke a width rule\nby rule: {by_check:?}\nby shape: {by_shape:?}\n\n{}",
            self.violations.len(),
            self.draws,
            examples.into_values().collect::<Vec<_>>().join("\n")
        );
    }
}

/// One shape, setup and width.
struct Case {
    shape: &'static str,
    setup: Setup,
    width: usize,
}

impl Case {
    fn label(&self) -> String {
        format!("{} {} w={}", self.shape, self.setup.label(), self.width)
    }
}

/// A bordered table read back from its text.
struct Drawn<'a> {
    lines: Vec<&'a str>,
    /// Column widths (padding included) read from the top border.
    widths: Vec<usize>,
    /// Whether the last column is the trailing `...` column.
    trail: bool,
    /// Whether the header row sits on the top border.
    on_border: bool,
}

impl<'a> Drawn<'a> {
    fn parse(out: &'a str) -> Option<Self> {
        let lines: Vec<&str> = out.lines().collect();
        let top = lines.first()?;
        if !top.starts_with('╭') {
            return None;
        }
        let widths = split_cells(top, '┬')
            .into_iter()
            .map(string_width)
            .collect();
        let first_row = lines.get(1)?;
        let last_cell = first_row.strip_suffix('│')?.rsplit('│').next()?.trim();
        let on_border = top.chars().any(|c| !"╭─┬╮".contains(c));
        Some(Self {
            lines,
            widths,
            trail: last_cell == "...",
            on_border,
        })
    }

    fn visible(&self) -> usize {
        self.widths.len() - self.trail as usize
    }

    /// Whether each column holds a nested table: whether a nested table's top
    /// border starts right after the left padding of one of its cells.
    fn nested_columns(&self, left_pad: usize) -> Vec<bool> {
        let mut starts = vec![];
        let mut at = 1;
        for width in &self.widths {
            starts.push(at + left_pad);
            at += width + 1;
        }

        let mut nested = vec![false; self.widths.len()];
        for line in &self.lines[1..] {
            let mut at = 0;
            for c in line.chars() {
                if c == '╭'
                    && let Some(col) = starts.iter().position(|&start| start == at)
                {
                    nested[col] = true;
                }
                at += get_char_width(c);
            }
        }
        nested
    }

    /// A line whose column borders do not line up with the top border. Only
    /// a table without nested tables is checked: their borders split cells too.
    fn misaligned_line(&self) -> Option<&'a str> {
        self.lines[1..].iter().copied().find(|line| {
            let separator = match line.chars().next() {
                Some('├') => '┼',
                Some('╰') => '┴',
                _ => '│',
            };
            let cells: Vec<usize> = split_cells(line, separator)
                .into_iter()
                .map(string_width)
                .collect();
            cells != self.widths
        })
    }
}

/// Checks one drawn table against the rules. Returns the column widths of a
/// bordered table, `None` for a table that did not render or has no borders.
fn check_layout(
    report: &mut Report,
    case: &Case,
    out: Option<&str>,
    spec: Option<&Spec>,
) -> Option<Vec<usize>> {
    report.draws += 1;
    let width = case.width;
    let out = out?;

    if let Some(line) = out.lines().find(|line| string_width(line) > width) {
        let line_width = string_width(line);
        report.push("fits", case, format!("line is {line_width} wide: {line}"));
    }

    let count = |line: &str, c: char| line.chars().filter(|&x| x == c).count();
    if let Some(line) = out
        .lines()
        .find(|l| count(l, '╭') != count(l, '╮') || count(l, '╰') != count(l, '╯'))
    {
        report.push("borders", case, format!("a box is cut: {line}"));
    }

    let spec = spec?;
    let Some(drawn) = Drawn::parse(out) else {
        report.push("parse", case, format!("unreadable table:\n{out}"));
        return None;
    };

    let top_width = string_width(drawn.lines[0]);
    if let Some(line) = drawn
        .lines
        .iter()
        .find(|line| string_width(line) != top_width)
    {
        let line_width = string_width(line);
        let text = format!("line is {line_width} wide, top border {top_width}: {line}");
        report.push("aligned", case, text);
    }

    // A drawn nested table keeps its width: the shrinking rules skip its column.
    let nested = if spec.stretches {
        drawn.nested_columns(case.setup.pad)
    } else {
        vec![false; drawn.widths.len()]
    };
    if !nested.contains(&true)
        && let Some(line) = drawn.misaligned_line()
    {
        let text = format!("column borders do not line up with the top border: {line}");
        report.push("junctions", case, text);
    }

    let widths = &drawn.widths;
    let natural = &spec.natural;
    let floors = &spec.floors(drawn.on_border);
    let count = natural.len();
    let detail = |text: String| format!("{text} (widths {widths:?}, natural {natural:?})");

    if bordered_width(natural) <= width {
        let total = bordered_width(natural);
        let stretched = spec.stretches && total * 5 > width * 4 && total < width;
        let ok = if stretched {
            widths.len() == count
                && widths.iter().zip(natural).all(|(w, n)| w >= n)
                && bordered_width(widths) == width
                && (!spec.index || widths[0] == natural[0])
        } else {
            widths == natural
        };
        if !ok {
            let text = format!("table fits but widths {widths:?} != natural {natural:?}");
            report.push("natural", case, text);
        }
        return Some(widths.clone());
    }

    let visible = drawn.visible();
    if visible > count || (visible < count) != drawn.trail {
        let detail = format!(
            "{visible} of {count} columns visible, trail={}",
            drawn.trail
        );
        report.push("trail", case, detail);
        return None;
    }

    let first_data = spec.first_data();
    if visible <= first_data {
        report.push("data", case, String::from("no data column visible"));
        return None;
    }
    let shrinks = |col: usize| !nested[col];
    // Only when not even the first data column fits at its floor is it squeezed.
    let squeezed = shrinks(first_data) && widths[first_data] < floors[first_data];
    if squeezed {
        let mut strict = floors[..=first_data].to_vec();
        if count > first_data + 1 {
            strict.push(TRAIL + 2 * case.setup.pad);
        }
        if bordered_width(&strict) <= width {
            let text = format!("column {first_data} squeezed though floors {strict:?} fit");
            report.push("squeezed", case, detail(text));
        }
    }

    for col in (0..visible).filter(|&col| shrinks(col)) {
        let floor = if squeezed && col == first_data {
            case.setup.squeezed(natural[col])
        } else {
            floors[col]
        };
        if widths[col] < floor {
            let text = format!("column {col} is {} < floor {floor}", widths[col]);
            report.push("floor", case, detail(text));
        }
    }

    // A column of a later tier is below its natural width only once every
    // column of the tiers before it sits at its floor.
    for col in (0..visible).filter(|&col| shrinks(col) && widths[col] < natural[col]) {
        let wide = (0..visible).find(|&other| {
            shrinks(other) && spec.tier(other) < spec.tier(col) && widths[other] > floors[other]
        });
        if let Some(other) = wide {
            let text = format!(
                "tier {} column {col} shrunk to {} while tier {} column {other} is {} > floor {}",
                spec.tier(col),
                widths[col],
                spec.tier(other),
                widths[other],
                floors[other]
            );
            report.push("tiers", case, detail(text));
        }
    }

    // Widest first: a shrunk plain column is at least as wide as every other
    // plain column, unless that column sits at its floor.
    let plain: Vec<usize> = (0..visible)
        .filter(|&col| shrinks(col) && spec.tier(col) == 0)
        .collect();
    for &i in plain.iter().filter(|&&col| widths[col] < natural[col]) {
        for &j in plain.iter().filter(|&&col| col != i) {
            if widths[j] > widths[i].max(floors[j]) + 1 {
                let text = format!(
                    "column {i} shrunk to {} while column {j} is {}",
                    widths[i], widths[j]
                );
                report.push("widest-first", case, detail(text));
            }
        }
    }

    // A visible column of nested tables may hide the columns to its right,
    // and a column of nested tables hides when drawing them would take a
    // visible priority column's third.
    let nested_claim = spec.nested.iter().any(|&col| col <= visible);
    if drawn.trail && !squeezed && !nested_claim {
        // The next column at its floor, plus `...` when more remain, must not
        // fit, or showing it must leave a visible priority column less than a
        // third of the width (counted in thirds, as the planner does).
        let pad = 2 * case.setup.pad;
        let mut needed: Vec<usize> = floors[..=visible].to_vec();
        if visible + 1 < count {
            needed.push(TRAIL + pad);
        }
        let room = 3 * width.saturating_sub(bordered_width(&needed) - needed.iter().sum::<usize>());
        let trail: usize = needed[visible + 1..].iter().sum();
        let wanted = 3 * trail
            + (0..=visible)
                .map(|col| match spec.priorities.contains(&col) {
                    true => (3 * natural[col]).min((3 * floors[col]).max(width)),
                    false => 3 * floors[col],
                })
                .sum::<usize>();
        if bordered_width(&needed) <= width && wanted <= room {
            let text = format!("column {visible} hidden though floors {needed:?} fit");
            report.push("hidden", case, detail(text));
        }
    }

    if drawn.on_border {
        for (col, header) in spec.headers.iter().take(visible).enumerate() {
            if !drawn.lines[0].contains(header.as_str()) {
                let text = format!("header {header:?} of column {col} is cut");
                report.push("header", case, text);
            }
        }
    }

    Some(widths.clone())
}

/// Checks the rules across the widths of one shape and setup: `draw(width,
/// marked)` draws the table at `width`, with or without its width priorities.
fn sweep(
    report: &mut Report,
    shape: &'static str,
    setup: Setup,
    widths: impl Iterator<Item = usize>,
    spec: Option<&Spec>,
    draw: impl Fn(usize, bool) -> Option<String>,
) {
    let has_priorities = spec.is_some_and(|spec| !spec.priorities.is_empty());
    // Visible columns and nested tables drawn at each narrower width.
    let mut narrower: Vec<(usize, usize)> = vec![];
    for width in widths {
        let case = Case {
            shape,
            setup,
            width,
        };
        let out = draw(width, true);
        if DETERMINISM_WIDTHS.contains(&width) && out != draw(width, true) {
            report.push("deterministic", &case, String::from("two draws differ"));
        }
        if out.is_none() && spec.is_some_and(|spec| width >= spec.render_width()) {
            report.push("renders", &case, String::from("no table"));
        }

        let Some(marked) = check_layout(report, &case, out.as_deref(), spec) else {
            continue;
        };

        // The visible columns, not counting the trailing `...` column, and
        // the nested tables drawn.
        let out = out.as_deref().unwrap_or_default();
        let shown = Drawn::parse(out).map_or(0, |drawn| drawn.visible());
        let nested = out.matches('╭').count().saturating_sub(1);
        if let Some(&(more, fewer_nested)) = narrower
            .iter()
            .find(|&&(visible, drawn)| visible > shown && drawn >= nested)
        {
            let text = format!(
                "{shown} columns and {nested} nested tables visible, \
                 {more} and {fewer_nested} at a narrower width"
            );
            report.push("monotone", &case, text);
        }
        narrower.push((shown, nested));

        let Some(spec) = spec.filter(|_| has_priorities) else {
            continue;
        };
        let Some(unmarked_out) = draw(width, false) else {
            continue;
        };
        let Some(unmarked) = Drawn::parse(&unmarked_out) else {
            continue;
        };
        for &col in spec
            .priorities
            .iter()
            .filter(|&&col| col < unmarked.visible())
        {
            if col >= shown || marked[col] < unmarked.widths[col] {
                let text = format!(
                    "priority column {col} is {:?} marked, {} unmarked",
                    marked.get(col).filter(|_| col < shown),
                    unmarked.widths[col]
                );
                report.push("priority", &case, text);
            }
        }
    }
}

// --- plain tables -----------------------------------------------------------

struct Shape {
    name: &'static str,
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    priorities: Vec<&'static str>,
    /// A record: keys in the first column, no header row.
    record: bool,
    /// The index column's values, row numbers when empty.
    index: Vec<String>,
}

impl Shape {
    fn new(name: &'static str, header: &[&str], rows: &[&[&str]]) -> Self {
        let rows = rows
            .iter()
            .map(|row| row.iter().map(|s| s.to_string()).collect())
            .collect();
        Self::owned(name, header, rows)
    }

    fn owned(name: &'static str, header: &[&str], rows: Vec<Vec<String>>) -> Self {
        Self {
            name,
            header: header.iter().map(|s| s.to_string()).collect(),
            rows,
            priorities: vec![],
            record: false,
            index: vec![],
        }
    }

    /// A record of `pairs`, drawn as `table` draws records.
    fn record(name: &'static str, pairs: &[(&str, &str)]) -> Self {
        let rows = pairs
            .iter()
            .map(|(key, value)| vec![key.to_string(), value.to_string()])
            .collect();
        Self {
            record: true,
            ..Self::owned(name, &[], rows)
        }
    }

    fn priorities(mut self, columns: &[&'static str]) -> Self {
        self.priorities = columns.to_vec();
        self
    }

    fn index(mut self, values: &[&str]) -> Self {
        self.index = values.iter().map(|s| s.to_string()).collect();
        self
    }

    fn with_index(&self, setup: Setup) -> bool {
        self.record || setup.index
    }

    /// Header plus data rows, with the index column first when the setup has one.
    fn data(&self, setup: Setup) -> Vec<Vec<String>> {
        if self.record {
            return self.rows.clone();
        }

        let index = setup.index;
        let mut header = self.header.clone();
        if index {
            header.insert(0, String::from("#"));
        }
        let mut data = vec![header];
        for (i, row) in self.rows.iter().enumerate() {
            let mut row = row.clone();
            if index {
                let value = self.index.get(i).cloned().unwrap_or_else(|| i.to_string());
                row.insert(0, value);
            }
            data.push(row);
        }
        data
    }

    fn priority_columns(&self, setup: Setup) -> Vec<usize> {
        self.priorities
            .iter()
            .filter_map(|name| self.header.iter().position(|h| h == name))
            .map(|col| col + setup.index as usize)
            .collect()
    }

    fn draw(&self, setup: Setup, width: usize, marked: bool) -> Option<String> {
        let data = self
            .data(setup)
            .into_iter()
            .map(|row| row.into_iter().map(Text::new).collect())
            .collect();
        let mut case = TestCase::new(width)
            .theme(if setup.bordered {
                TableTheme::rounded()
            } else {
                TableTheme::none()
            })
            .trim(setup.strategy())
            .indent(setup.indent())
            .header_on_border(setup.header_on_border);
        if marked {
            case = case.priorities(self.priority_columns(setup));
        }
        if !self.record {
            case = case.header();
        }
        if self.with_index(setup) {
            case = case.index();
        }
        create_table(data, case)
    }

    fn spec(&self, setup: Setup) -> Spec {
        let data = self.data(setup);
        let pad = 2 * setup.pad;
        let natural: Vec<usize> = (0..data[0].len())
            .map(|col| {
                data.iter()
                    .map(|row| string_width(&row[col]))
                    .max()
                    .unwrap_or(0)
                    + pad
            })
            .collect();
        Spec {
            natural,
            index: self.with_index(setup),
            priorities: self.priority_columns(setup),
            nested: vec![],
            headers: if self.record { vec![] } else { data[0].clone() },
            stretches: false,
            setup,
        }
    }
}

fn plain_shapes() -> Vec<Shape> {
    let path = path_entries().join(":");
    let w22 = |c: char| c.to_string().repeat(22);
    let long_key = "k".repeat(100);

    vec![
        Shape::new(
            "env",
            &["key", "value"],
            &[
                &["AI_AGENT", "claude-code_2-1-303_agent"],
                &["CLAUDE_CODE_CHILD_SESSION", "1"],
                &["FPATH", &path],
                &["HOME", "/Users/fdncred"],
                &["LS_COLORS", &"di=01;34:ln=01;36:".repeat(20)],
                &["TERM", "xterm-256color"],
            ],
        ),
        Shape::new(
            "ls",
            &["name", "type", "size", "modified"],
            &[
                &[LONG_NAME, "file", "1.2 kB", "2 years ago"],
                &["Cargo.toml", "file", "9.8 kB", "a day ago"],
                &["crates", "dir", "1.5 kB", "3 hours ago"],
            ],
        )
        .priorities(&["name"]),
        Shape::new(
            "ls_long",
            &[
                "name",
                "type",
                "target",
                "readonly",
                "mode",
                "num_links",
                "inode",
                "user",
                "group",
                "size",
                "created",
                "accessed",
                "modified",
            ],
            &[&[
                "grok-continue-115-caching-regression.txt",
                "file",
                "",
                "false",
                "rw-r--r--",
                "1",
                "216175737",
                "fdncred",
                "staff",
                "128 B",
                "2 years ago",
                "2 years ago",
                "2 years ago",
            ]],
        ),
        Shape::new(
            "ps",
            &[
                "pid", "ppid", "name", "status", "cpu", "mem", "virtual", "command",
            ],
            &[
                &[
                    "9817",
                    "1",
                    "Google Chrome Helper (Renderer)",
                    "Sleep",
                    "0.12",
                    "112.3 MB",
                    "1.6 TB",
                    "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Helpers/Google Chrome Helper (Renderer).app --type=renderer",
                ],
                &[
                    "1", "0", "launchd", "Sleep", "0.00", "12.0 MB", "420.1 GB", "/sbin/launchd",
                ],
            ],
        )
        .priorities(&["command", "name"]),
        Shape::owned(
            "priority_last",
            &["a", "b", "c", "d", "e", "f", "g", "h", "command"],
            vec![
                "abcdefgh"
                    .chars()
                    .map(|c| c.to_string().repeat(8))
                    .chain([format!("/usr/bin/cmd --flag {}", "x".repeat(180))])
                    .collect(),
            ],
        )
        .priorities(&["command"]),
        Shape::owned(
            "equal8",
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            vec![
                "abcdefgh".chars().map(w22).collect(),
                "abcdefgh".chars().map(w22).collect(),
            ],
        ),
        Shape::owned(
            "equal8_priority",
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            vec!["abcdefgh".chars().map(w22).collect()],
        )
        .priorities(&["g"]),
        Shape::owned(
            "two_priorities",
            &["a", "b", "c", "d", "e", "f", "g", "h"],
            vec!["abcdefgh".chars().map(|c| c.to_string().repeat(50)).collect()],
        )
        .priorities(&["c", "g"]),
        Shape::new(
            "wide_first",
            &["description", "a", "bb", "ccc"],
            &[
                &[&WORDS.repeat(2), "x", "yes", "12.5"],
                &["short", "y", "no", "7"],
            ],
        ),
        Shape::new(
            "wide_last",
            &["a", "bb", "ccc", "description"],
            &[
                &["x", "yes", "12.5", &WORDS.repeat(2)],
                &["y", "no", "7", "short"],
            ],
        ),
        Shape::new(
            "two_wide",
            &["left", "mid", "right"],
            &[&[WORDS, "-", &WORDS.repeat(2)]],
        ),
        Shape::new(
            "squeezed_first",
            &["description", "b", "c"],
            &[&[&"x".repeat(50), "1", "2"]],
        ),
        Shape::new("single", &["text"], &[&[&WORDS.repeat(3)], &["short"]]),
        Shape::new(
            "cjk",
            &["name", "value"],
            &[
                &["漢字表漢字表漢字表漢字表漢字表漢字表", "表"],
                &["ascii", &"漢字".repeat(30)],
            ],
        ),
        Shape::new(
            "cjk_header",
            &["名前", "説明", "🙂"],
            &[
                &["漢字表漢字表", &WORDS.repeat(2), "12"],
                &["ascii", "short", "7"],
            ],
        ),
        Shape::new(
            "cjk_header_last",
            &["a", "b", "c", "名前"],
            &[&[&"a".repeat(15), &"b".repeat(16), &"c".repeat(15), "x"]],
        ),
        Shape::new(
            "ligature_header",
            &["الاسلام", "الاسم", "b"],
            &[&["1", "2", &WORDS.repeat(2)]],
        ),
        Shape::new(
            "emoji",
            &["mood", "note"],
            &[&["😀😀😀", &"🎉 party ".repeat(12)], &["🙂", "calm"]],
        ),
        Shape::new(
            "empty_cells",
            &["a", "b", "c"],
            &[&["", "", ""], &["x", "", &WORDS.repeat(2)]],
        ),
        Shape::new(
            "multiline",
            &["key", "lines"],
            &[
                &[
                    "poem",
                    "first line\nsecond, a much longer line than the first one\nthird",
                ],
                &["one", "single"],
            ],
        ),
        Shape::new(
            "long_headers",
            &["process_threads", "virtual_memory_bytes", "command"],
            &[&["4", "1", &WORDS.repeat(2)]],
        ),
        Shape::new("long_index", &["a"], &[&["1"], &["2"]]).index(&[&"x".repeat(100), "y"]),
        Shape::record(
            "record",
            &[
                ("name", "nu"),
                ("description", &WORDS.repeat(2)),
                ("home", "/Users/fdncred"),
            ],
        ),
        Shape::record("record_long_key", &[(&long_key, "1"), ("b", "2")]),
    ]
}

#[test]
fn plain_tables_follow_the_width_rules() {
    run_sweeps(
        &plain_shapes(),
        &Setup::all(&[true, false]),
        |shape, setup| {
            let mut report = Report::default();
            let spec = setup.bordered.then(|| shape.spec(setup));
            let draw = |width, marked| shape.draw(setup, width, marked);
            sweep(
                &mut report,
                shape.name,
                setup,
                sweep_widths(10),
                spec.as_ref(),
                draw,
            );
            report
        },
    );
}

// --- expanded tables --------------------------------------------------------

struct ValueShape {
    name: &'static str,
    value: Value,
    priorities: Vec<&'static str>,
}

impl ValueShape {
    fn new(name: &'static str, value: Value) -> Self {
        Self {
            name,
            value,
            priorities: vec![],
        }
    }

    fn priorities(mut self, columns: &[&'static str]) -> Self {
        self.priorities = columns.to_vec();
        self
    }

    fn is_record(&self) -> bool {
        matches!(self.value, Value::Record { .. })
    }

    /// The columns `table --expand` draws holding nested lists or records.
    fn nested_columns(&self, setup: Setup) -> Vec<usize> {
        let is_nested = |value: &Value| match value {
            Value::List { vals, .. } => !vals.is_empty(),
            Value::Record { val, .. } => !val.is_empty(),
            _ => false,
        };
        match &self.value {
            Value::Record { val, .. } => {
                let nested = val.values().any(is_nested);
                if nested { vec![1] } else { vec![] }
            }
            Value::List { vals, .. } => {
                let index = usize::from(setup.index);
                let headers = get_columns(vals);
                if headers.is_empty() {
                    return if vals.iter().any(is_nested) {
                        vec![index]
                    } else {
                        vec![]
                    };
                }
                (0..headers.len())
                    .filter(|&col| {
                        vals.iter().any(|row| {
                            row.as_record()
                                .ok()
                                .and_then(|record| record.get(&headers[col]))
                                .is_some_and(is_nested)
                        })
                    })
                    .map(|col| col + index)
                    .collect()
            }
            _ => vec![],
        }
    }

    /// The header row `table --expand` draws.
    fn headers(&self, setup: Setup) -> Vec<String> {
        let Value::List { vals, .. } = &self.value else {
            return vec![];
        };
        let mut headers = get_columns(vals);
        if setup.index && !headers.is_empty() {
            headers.insert(0, String::from("#"));
        }
        headers
    }

    fn draw(&self, drawer: &Drawer, width: usize, marked: bool) -> Result<Option<String>, String> {
        let priorities = if marked {
            self.priorities.iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        let signals = Signals::empty();
        let opts = TableOpts::new(
            &drawer.config,
            StyleComputer::new(drawer.engine_state, drawer.stack, Default::default()),
            &signals,
            Span::test_data(),
            width,
            TableMode::Rounded,
            0,
            false,
            priorities,
        );

        let table = ExpandedTable::new(None, false, String::from(" "));
        let out = match &self.value {
            Value::List { vals, .. } => table.build_list(vals, opts),
            Value::Record { val, .. } => table.build_map(val, opts),
            _ => unreachable!("shapes are lists or records"),
        };
        out.map_err(|err| format!("{err:?}"))
    }

    fn spec(&self, drawer: &Drawer) -> Option<Spec> {
        let natural_out = self.draw(drawer, 4000, true).ok()??;
        let natural = Drawn::parse(&natural_out)?.widths;
        let setup = drawer.setup;
        let headers = self.headers(setup);
        let priorities = self
            .priorities
            .iter()
            .filter_map(|name| headers.iter().position(|h| h == name))
            .collect();
        Some(Spec {
            natural,
            index: self.is_record() || setup.index,
            priorities,
            nested: self.nested_columns(setup),
            headers,
            stretches: true,
            setup,
        })
    }
}

/// What an expanded table is drawn with, for one setup.
struct Drawer<'a> {
    setup: Setup,
    config: Config,
    engine_state: &'a EngineState,
    stack: &'a Stack,
}

impl<'a> Drawer<'a> {
    fn new(setup: Setup, engine_state: &'a EngineState, stack: &'a Stack) -> Self {
        let mut config = Config::default();
        config.table.trim = setup.strategy();
        config.table.header_on_separator = setup.header_on_border;
        config.table.padding = setup.indent();
        config.table.index_mode = if setup.index {
            TableIndexMode::Always
        } else {
            TableIndexMode::Never
        };
        Self {
            setup,
            config,
            engine_state,
            stack,
        }
    }
}

fn record(pairs: Vec<(&str, Value)>) -> Value {
    Value::test_record(Record::from_iter(
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)),
    ))
}

fn strings(items: &[&str]) -> Value {
    Value::test_list(items.iter().map(|s| Value::test_string(*s)).collect())
}

fn s(text: &str) -> Value {
    Value::test_string(text)
}

fn expanded_shapes() -> Vec<ValueShape> {
    let path = path_entries();
    let conversions = record(vec![
        (
            "LS_COLORS",
            record(vec![
                ("to_string", s("{ |v| $v | str join (char esep) }")),
                ("from_string", s("{ |s| $s | split row (char esep) }")),
            ]),
        ),
        (
            "DYLD_FALLBACK_LIBRARY_PATH",
            record(vec![
                (
                    "to_string",
                    s("{ |v| $v | path expand | str join (char esep) }"),
                ),
                ("from_string", s("{ |s| $s | split row (char esep) }")),
            ]),
        ),
    ]);
    let env_row = |key: &str, value: Value| record(vec![("key", s(key)), ("value", value)]);
    let plugin = |name: &str, version: &str, file: &str, commands: Vec<(&str, &str)>| {
        record(vec![
            ("name", s(name)),
            ("version", s(version)),
            ("status", s("loaded")),
            ("pid", Value::test_nothing()),
            ("filename", s(file)),
            ("shell", Value::test_nothing()),
            (
                "commands",
                Value::test_list(
                    commands
                        .into_iter()
                        .map(|(n, d)| record(vec![("name", s(n)), ("description", s(d))]))
                        .collect(),
                ),
            ),
        ])
    };
    let plugins = Value::test_list(vec![
        plugin(
            "dns",
            "4.0.13-alpha.1",
            r"C:\Users\x\.cargo\bin\nu_plugin_dns.exe",
            vec![("dns query", "Perform a DNS query")],
        ),
        plugin(
            "file",
            "0.27.0",
            r"C:\Users\x\.cargo\bin\nu_plugin_file.exe",
            vec![
                ("file", "View file format information"),
                ("file info", "Show everything the plugin knows about a file"),
            ],
        ),
    ]);
    let wide_row = |i: i64| {
        record(
            (0..6)
                .map(|c| {
                    let key: &'static str = ["c0", "c1", "c2", "c3", "c4", "c5"][c];
                    (key, s(&format!("value_{i}_abcdefghijklmn")))
                })
                .collect(),
        )
    };
    let numbers = || Value::test_list((1..=3).map(Value::test_int).collect());
    let deep = (0..30).fold(Value::test_list(vec![]), |acc, _| {
        Value::test_list(vec![acc])
    });

    vec![
        ValueShape::new(
            "env",
            Value::test_list(vec![
                env_row("AI_AGENT", s("claude-code_2-1-303_agent")),
                env_row("CLAUDE_CODE_CHILD_SESSION", s("1")),
                env_row("ENV_CONVERSIONS", conversions),
                env_row("FPATH", s(&path.join(":"))),
                env_row("HOME", s("/Users/fdncred")),
                env_row("NU_LIB_DIRS", strings(&path[..4])),
                env_row("PATH", strings(&path)),
                env_row("TERM", s("xterm-256color")),
            ]),
        ),
        ValueShape::new("plugin_list", plugins.clone()),
        ValueShape::new("plugin_list_priority", plugins).priorities(&["filename"]),
        ValueShape::new(
            "cargo_toml",
            record(vec![
                (
                    "package",
                    record(vec![
                        ("name", s("nu")),
                        ("version", s("0.116.2")),
                        ("description", s(&WORDS.repeat(2))),
                    ]),
                ),
                (
                    "dependencies",
                    record(vec![
                        ("chrono", s("0.4.42")),
                        (
                            "tabled",
                            record(vec![
                                ("version", s("0.20.0")),
                                ("features", strings(&["ansi", "std"])),
                                ("default-features", Value::test_bool(false)),
                            ]),
                        ),
                    ]),
                ),
                (
                    "workspace",
                    record(vec![(
                        "members",
                        strings(&["crates/nu-cli", "crates/nu-table", "crates/nu-command"]),
                    )]),
                ),
            ]),
        ),
        ValueShape::new(
            "nested_middle",
            Value::test_list(vec![
                record(vec![
                    ("name", s("alpha")),
                    (
                        "details",
                        Value::test_list(vec![
                            record(vec![("key", s("size")), ("value", s("1.2 kB"))]),
                            record(vec![("key", s("owner")), ("value", s("fdncred"))]),
                        ]),
                    ),
                    ("description", s(&WORDS.repeat(2))),
                ]),
                record(vec![
                    ("name", s("beta")),
                    ("details", Value::test_list(vec![])),
                    ("description", s("short")),
                ]),
            ]),
        ),
        ValueShape::new(
            "dependencies",
            Value::test_list(vec![record(vec![
                ("name", s("pkg")),
                ("description", s(&WORDS.repeat(2))),
                (
                    "dependencies",
                    record(vec![
                        ("@typescript-eslint/eslint-plugin", s("^5.0.0")),
                        ("eslint-config-prettier", s("^8.0.0")),
                    ]),
                ),
                ("x", Value::test_int(1)),
            ])]),
        ),
        ValueShape::new(
            "nested_fallback",
            Value::test_list(vec![
                record(vec![
                    ("k", s("x")),
                    (
                        "v",
                        record(vec![
                            (&"a".repeat(120), Value::test_int(1)),
                            ("b", Value::test_int(2)),
                        ]),
                    ),
                ]),
                record(vec![("k", s("y")), ("v", Value::test_int(1))]),
            ]),
        ),
        ValueShape::new(
            "deep",
            Value::test_list(vec![record(vec![
                ("id", Value::test_int(1)),
                (
                    "a",
                    record(vec![(
                        "b",
                        record(vec![(
                            "c",
                            Value::test_list(vec![record(vec![
                                ("d", s(WORDS)),
                                ("e", strings(&["x", "y"])),
                            ])]),
                        )]),
                    )]),
                ),
            ])]),
        ),
        ValueShape::new("deep_lists", deep),
        ValueShape::new(
            "list_of_lists",
            Value::test_list(vec![strings(&["a", "b", "c"]), strings(&[WORDS, "short"])]),
        ),
        ValueShape::new(
            "wide_nested",
            Value::test_list(vec![record(vec![
                ("id", Value::test_int(7)),
                ("rows", Value::test_list((0..3).map(wide_row).collect())),
            ])]),
        ),
        ValueShape::new(
            "nested_cells",
            Value::test_list(
                (0..3)
                    .map(|i| {
                        record(vec![
                            ("id", Value::test_int(i)),
                            (
                                "inner",
                                Value::test_list(
                                    (0..3)
                                        .map(|_| {
                                            record(
                                                ["c0", "c1", "c2", "c3", "c4", "c5"]
                                                    .into_iter()
                                                    .map(|key| (key, numbers()))
                                                    .collect(),
                                            )
                                        })
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        ValueShape::new(
            "cjk",
            Value::test_list(vec![record(vec![
                ("name", s("漢字表漢字表漢字表")),
                (
                    "data",
                    Value::test_list(vec![record(vec![
                        ("k", s("表")),
                        ("v", s(&"漢字".repeat(25))),
                    ])]),
                ),
            ])]),
        ),
        ValueShape::new(
            "cjk_header",
            Value::test_list(vec![record(vec![
                ("名前", s("漢字表漢字表漢字表")),
                (
                    "data",
                    Value::test_list(vec![record(vec![
                        ("鍵", s("表")),
                        ("値", s(&"漢字".repeat(25))),
                    ])]),
                ),
            ])]),
        ),
        ValueShape::new(
            "priority",
            Value::test_list(
                (0..2)
                    .map(|_| {
                        record(vec![
                            ("a", s(&"a".repeat(60))),
                            ("b", s(&"b".repeat(90))),
                            ("c", s(&"c".repeat(60))),
                        ])
                    })
                    .collect(),
            ),
        )
        .priorities(&["b"]),
        ValueShape::new(
            "record_long_keys",
            record(vec![
                ("a_rather_long_configuration_key_name", s(&WORDS.repeat(2))),
                ("short", strings(&["one", "two"])),
            ]),
        ),
        ValueShape::new(
            "empty_nested",
            Value::test_list(vec![record(vec![
                ("a", Value::test_list(vec![])),
                ("b", record(vec![])),
                ("c", s(WORDS)),
            ])]),
        ),
        ValueShape::new(
            "scalars",
            Value::test_list(vec![Value::test_int(1), s(&WORDS.repeat(2)), s("short")]),
        ),
    ]
}

#[test]
fn expanded_tables_follow_the_width_rules() {
    // Expanded tables plan like plain ones, so wrapping at any character adds
    // nothing to the plain sweep here.
    let setups: Vec<Setup> = Setup::all(&[true])
        .into_iter()
        .filter(|setup| !matches!(setup.trim, Trim::WrapAnyChar))
        .collect();
    let engine_state = EngineState::new();
    let stack = Stack::new();

    run_sweeps(&expanded_shapes(), &setups, |shape, setup| {
        let mut report = Report::default();
        let drawer = Drawer::new(setup, &engine_state, &stack);
        let spec = shape.spec(&drawer);
        let draw = |width, marked| match shape.draw(&drawer, width, marked) {
            Ok(out) => out,
            Err(err) => Some(format!("error: {err}")),
        };
        sweep(
            &mut report,
            shape.name,
            setup,
            sweep_widths(20),
            spec.as_ref(),
            draw,
        );
        report
    });
}
