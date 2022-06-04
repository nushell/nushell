use crate::query_web::css;
use scraper::{element_ref::ElementRef, Html, Selector as ScraperSelector};
use std::collections::HashMap;

pub type Headers = HashMap<String, usize>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebTable {
    headers: Headers,
    pub data: Vec<Vec<String>>,
}

impl WebTable {
    /// Finds the first table in `html`.
    pub fn find_first(html: &str) -> Option<WebTable> {
        let html = Html::parse_fragment(html);
        html.select(&css("table", false)).next().map(WebTable::new)
    }

    pub fn find_all_tables(html: &str) -> Option<Vec<WebTable>> {
        let html = Html::parse_fragment(html);
        let iter: Vec<WebTable> = html
            .select(&css("table", false))
            .map(WebTable::new)
            .collect();
        if iter.is_empty() {
            return None;
        }
        Some(iter)
    }

    /// Finds the table in `html` with an id of `id`.
    pub fn find_by_id(html: &str, id: &str) -> Option<WebTable> {
        let html = Html::parse_fragment(html);
        let selector = format!("table#{}", id);
        ScraperSelector::parse(&selector)
            .ok()
            .as_ref()
            .map(|s| html.select(s))
            .and_then(|mut s| s.next())
            .map(WebTable::new)
    }

    /// Finds the table in `html` whose first row contains all of the headers
    /// specified in `headers`. The order does not matter.
    ///
    /// If `headers` is empty, this is the same as
    /// [`find_first`](#method.find_first).
    pub fn find_by_headers<T>(html: &str, headers: &[T]) -> Option<Vec<WebTable>>
    where
        T: AsRef<str>,
    {
        if headers.is_empty() {
            return WebTable::find_all_tables(html);
        }

        let sel_table = css("table", false);
        let sel_tr = css("tr", false);
        let sel_th = css("th", false);

        let html = Html::parse_fragment(html);
        let mut tables = html
            .select(&sel_table)
            .filter(|table| {
                table.select(&sel_tr).next().map_or(false, |tr| {
                    let cells = select_cells(tr, &sel_th, true);
                    headers.iter().all(|h| contains_str(&cells, h.as_ref()))
                })
            })
            .peekable();
        tables.peek()?;
        Some(tables.map(WebTable::new).collect())
    }

    /// Returns the headers of the table.
    ///
    /// This will be empty if the table had no `<th>` tags in its first row. See
    /// [`Headers`](type.Headers.html) for more.
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns an iterator over the [`Row`](struct.Row.html)s of the table.
    ///
    /// Only `<td>` cells are considered when generating rows. If the first row
    /// of the table is a header row, meaning it contains at least one `<th>`
    /// cell, the iterator will start on the second row. Use
    /// [`headers`](#method.headers) to access the header row in that case.
    pub fn iter(&self) -> Iter {
        Iter {
            headers: &self.headers,
            iter: self.data.iter(),
        }
    }

    pub fn empty() -> WebTable {
        WebTable {
            headers: HashMap::new(),
            data: vec![vec!["".to_string()]],
        }
    }

    // fn new(element: ElementRef) -> Table {
    //     let sel_tr = css("tr", false);
    //     let sel_th = css("th", false);
    //     let sel_td = css("td", false);

    //     let mut headers = HashMap::new();
    //     let mut rows = element.select(&sel_tr).peekable();
    //     if let Some(tr) = rows.peek() {
    //         for (i, th) in tr.select(&sel_th).enumerate() {
    //             headers.insert(cell_content(th), i);
    //         }
    //     }
    //     if !headers.is_empty() {
    //         rows.next();
    //     }
    //     let data = rows.map(|tr| select_cells(tr, &sel_td, true)).collect();
    //     Table { headers, data }
    // }

    fn new(element: ElementRef) -> WebTable {
        let sel_tr = css("tr", false);
        let sel_th = css("th", false);
        let sel_td = css("td", false);

        let mut headers = HashMap::new();
        let mut rows = element.select(&sel_tr).peekable();
        if let Some(tr) = rows.clone().peek() {
            for (i, th) in tr.select(&sel_th).enumerate() {
                headers.insert(cell_content(th), i);
            }
        }
        if !headers.is_empty() {
            rows.next();
        }

        if headers.is_empty() {
            // try looking for data as headers i.e. they're row headers not column headers
            for (i, d) in rows
                .clone()
                .map(|tr| select_cells(tr, &sel_th, true))
                .enumerate()
            {
                headers.insert(d.join(", "), i);
            }
            // check if headers are there but empty
            let mut empty_headers = true;
            for (h, _i) in headers.clone() {
                if !h.is_empty() {
                    empty_headers = false;
                    break;
                }
            }
            if empty_headers {
                headers = HashMap::new();
            }
            let data = rows.map(|tr| select_cells(tr, &sel_td, true)).collect();
            WebTable { headers, data }
        } else {
            let data = rows.map(|tr| select_cells(tr, &sel_td, true)).collect();
            WebTable { headers, data }
        }
    }
}

impl<'a> IntoIterator for &'a WebTable {
    type Item = Row<'a>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the rows in a [`Table`](struct.Table.html).
pub struct Iter<'a> {
    headers: &'a Headers,
    iter: std::slice::Iter<'a, Vec<String>>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Row<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let headers = self.headers;
        self.iter.next().map(|cells| Row { headers, cells })
    }
}

/// A row in a [`Table`](struct.Table.html).
///
/// A row consists of a number of data cells stored as strings. If the row
/// contains the same number of cells as the table's header row, its cells can
/// be safely accessed by header names using [`get`](#method.get). Otherwise,
/// the data should be accessed via [`as_slice`](#method.as_slice) or by
/// iterating over the row.
///
/// This struct can be thought of as a lightweight reference into a table. As
/// such, it implements the `Copy` trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Row<'a> {
    headers: &'a Headers,
    cells: &'a [String],
}

impl<'a> Row<'a> {
    /// Returns the number of cells in the row.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns `true` if the row contains no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns the cell underneath `header`.
    ///
    /// Returns `None` if there is no such header, or if there is no cell at
    /// that position in the row.
    pub fn get(&self, header: &str) -> Option<&'a str> {
        // eprintln!(
        //     "header={}, headers={:?}, cells={:?}",
        //     &header, &self.headers, &self.cells
        // );
        self.headers.get(header).and_then(|&i| {
            // eprintln!("i={}", i);
            self.cells.get(i).map(String::as_str)
        })
    }

    pub fn get_header_at(&self, index: usize) -> Option<&'a str> {
        let mut a_match = "";
        for (key, val) in self.headers {
            if *val == index {
                a_match = key;
                break;
            }
        }
        if a_match.is_empty() {
            None
        } else {
            Some(a_match)
        }
    }

    /// Returns a slice containing all the cells.
    pub fn as_slice(&self) -> &'a [String] {
        self.cells
    }

    /// Returns an iterator over the cells of the row.
    pub fn iter(&self) -> std::slice::Iter<String> {
        self.cells.iter()
    }
}

impl<'a> IntoIterator for Row<'a> {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

fn select_cells(
    element: ElementRef,
    selector: &ScraperSelector,
    remove_html_tags: bool,
) -> Vec<String> {
    if remove_html_tags {
        let scraped = element.select(selector).map(cell_content);
        let mut dehtmlized: Vec<String> = Vec::new();
        for item in scraped {
            let frag = Html::parse_fragment(&item);
            for node in frag.tree {
                if let scraper::node::Node::Text(text) = node {
                    dehtmlized.push(text.text.to_string());
                }
            }
        }
        dehtmlized
    } else {
        element.select(selector).map(cell_content).collect()
    }
}

fn cell_content(element: ElementRef) -> String {
    // element.inner_html().trim().to_string()
    let mut dehtmlize = String::new();
    let element = element.inner_html().trim().to_string();
    let frag = Html::parse_fragment(&element);
    for node in frag.tree {
        if let scraper::node::Node::Text(text) = node {
            dehtmlize.push_str(&text.text)
        }
    }

    // eprintln!("element={} dehtmlize={}", &element, &dehtmlize);

    if dehtmlize.is_empty() {
        dehtmlize = element;
    }

    dehtmlize
}

fn contains_str(slice: &[String], item: &str) -> bool {
    // slice.iter().any(|s| s == item)

    let mut dehtmlized = String::new();
    let frag = Html::parse_fragment(item);
    for node in frag.tree {
        if let scraper::node::Node::Text(text) = node {
            dehtmlized.push_str(&text.text);
        }
    }

    if dehtmlized.is_empty() {
        dehtmlized = item.to_string();
    }

    slice.iter().any(|s| {
        // eprintln!(
        //     "\ns={} item={} contains={}\n",
        //     &s,
        //     &dehtmlized,
        //     &dehtmlized.contains(s)
        // );
        // s.starts_with(item)
        dehtmlized.contains(s)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::query_web::retrieve_tables;
    // use indexmap::indexmap;
    // use nu_protocol::Value;

    const TABLE_EMPTY: &str = r#"
<table></table>
"#;

    const TABLE_TH: &str = r#"
<table>
    <tr><th>Name</th><th>Age</th></tr>
</table>
"#;

    const TABLE_TD: &str = r#"
<table>
    <tr><td>Name</td><td>Age</td></tr>
</table>
"#;

    const TWO_TABLES_TD: &str = r#"
<table>
    <tr><td>Name</td><td>Age</td></tr>
</table>
<table>
    <tr><td>Profession</td><td>Civil State</td></tr>
</table>
"#;

    const TABLE_TH_TD: &str = r#"
<table>
    <tr><th>Name</th><th>Age</th></tr>
    <tr><td>John</td><td>20</td></tr>
</table>
"#;

    const TWO_TABLES_TH_TD: &str = r#"
<table>
    <tr><th>Name</th><th>Age</th></tr>
    <tr><td>John</td><td>20</td></tr>
</table>
<table>
    <tr><th>Profession</th><th>Civil State</th></tr>
    <tr><td>Mechanic</td><td>Single</td></tr>
</table>
"#;

    const TABLE_TD_TD: &str = r#"
<table>
    <tr><td>Name</td><td>Age</td></tr>
    <tr><td>John</td><td>20</td></tr>
</table>
"#;

    const TABLE_TH_TH: &str = r#"
<table>
    <tr><th>Name</th><th>Age</th></tr>
    <tr><th>John</th><th>20</th></tr>
</table>
"#;

    const TABLE_COMPLEX: &str = r#"
<table>
    <tr><th>Name</th><th>Age</th><th>Extra</th></tr>
    <tr><td>John</td><td>20</td></tr>
    <tr><td>May</td><td>30</td><td>foo</td></tr>
    <tr></tr>
    <tr><td>a</td><td>b</td><td>c</td><td>d</td></tr>
</table>
"#;

    const TWO_TABLES_COMPLEX: &str = r#"
<!doctype HTML>
<html>
    <head><title>foo</title></head>
    <body>
        <table>
            <tr><th>Name</th><th>Age</th><th>Extra</th></tr>
            <tr><td>John</td><td>20</td></tr>
            <tr><td>May</td><td>30</td><td>foo</td></tr>
            <tr></tr>
            <tr><td>a</td><td>b</td><td>c</td><td>d</td></tr>
        </table>
        <table>
            <tr><th>Profession</th><th>Civil State</th><th>Extra</th></tr>
            <tr><td>Carpenter</td><td>Single</td></tr>
            <tr><td>Mechanic</td><td>Married</td><td>bar</td></tr>
            <tr></tr>
            <tr><td>e</td><td>f</td><td>g</td><td>h</td></tr>
        </table>
    </body>
</html>
"#;

    const HTML_NO_TABLE: &str = r#"
<!doctype HTML>
<html>
    <head><title>foo</title></head>
    <body><p>Hi.</p></body>
</html>
"#;

    const HTML_TWO_TABLES: &str = r#"
<!doctype HTML>
<html>
    <head><title>foo</title></head>
    <body>
        <table id="first">
            <tr><th>Name</th><th>Age</th></tr>
            <tr><td>John</td><td>20</td></tr>
        </table>
        <table id="second">
            <tr><th>Name</th><th>Weight</th></tr>
            <tr><td>John</td><td>150</td></tr>
        </table>
    </body>
</html>
"#;

    const HTML_TABLE_FRAGMENT: &str = r#"
        <table id="first">
            <tr><th>Name</th><th>Age</th></tr>
            <tr><td>John</td><td>20</td></tr>
        </table>
    </body>
</html>
"#;

    /*
        const HTML_TABLE_WIKIPEDIA_WITH_COLUMN_NAMES: &str = r#"
        <table class="wikitable">
        <caption>Excel 2007 formats
        </caption>
        <tbody><tr>
        <th>Format
        </th>
        <th>Extension
        </th>
        <th>Description
        </th></tr>
        <tr>
        <td>Excel Workbook
        </td>
        <td><code class="mw-highlight mw-highlight-lang-text mw-content-ltr" id="" style="" dir="ltr">.xlsx</code>
        </td>
        <td>The default Excel 2007 and later workbook format. In reality, a <a href="/wiki/Zip_(file_format)" class="medirect" title="Zip (file format)">Zip</a> compressed archive with a directory structure of <a href="/wiki/XML" title="XML">XML</a> text documents.Functions as the primary replacement for the former binary .xls format, although it does not support Excel macroor security reasons. Saving as .xlsx offers file size reduction over .xls<sup id="cite_ref-38" class="referencea href="&#35;cite_note-38">[38]</a></sup>
        </td></tr>     <tr>
        <td>Excel ro-enabled Workbook
        </td>     <td><code class="mw-highlight mw-highlight-lang-text mw-content-ltr" id="" style="" dir="ltr">.xlsm<de>     </td>
        <As Excel Workbook, but with macro support.
        <></tr>
        <
        <Excel Binary Workbook
        <>
        <<code class="mw-highlight mw-highlight-lang-text mw-content-ltr" id="" style="" dir="ltr">.xlsb</code>
        <>
        <As Excel Macro-enabled Workbook, but storing information in binary form rather than XML documents for openingd ing documents more quickly and efficiently. Intended especially for very large documents with tens of thousands s, and/or several hundreds
    f umns. This format is very useful for shrinking large Excel files as is often the case when doing data analysis.  </td></tr>
     <tr>
     <td>Excel Macro-enabled Template
     </td>
        <td><code class="mw-highlight mw-highlight-lang-text mw-content-ltr" id="" style="" dir="ltr">.xltm</code>
        </td>
        <td>A template document that forms a basis for actual workbooks, with macro support. The replacement for the o.xlt format.
        </td></tr>     <tr>
        <td>Excel -in
        </td>     <td><code class="mw-highlight mw-highlight-lang-text mw-content-ltr" id="" style="" dir="ltr">.xlam<de>     </td>
        <Excel add-in to add extra functionality and tools. Inherent macro support because of the file purpose.
        <></tr></tbody></table>
        "
        ct HTML_TABLE_WIKIPEDIA_COLUMNS_AS_ROWS: &str = r#"
    <tabllass="infobox vevent">
      <caon class="infobox-title summary">
        Mosoft Excel
      </cion>
      <tb>
        <
         d colspan="2" class="infobox-image">
         <a
           href="/wiki/File:Microsoft_Office_Excel_(2019%E2%80%93present).svg"
           class="image"
           ><img
             alt="Microsoft Office Excel (2019–present).svg"
             src="//upload.wikimedia.org/wikipedia/commons/thumb/3/34/Microsoft_Office_Excel_%282019%E2%80%93present%2vgpx-Microsoft_Office_Excel_%282019%E2%80%93present%29.svg.png"
              decoding="async"          width="69"
              height="64"          srcset="
                //upload.imedia.org/wikipedia/commons/thumb/3/34/Microsoft_Office_Excel_%282019%E2%80%93present%29.svgx-Microsoft_Office_el_%282019%E2%80%93present%29.svg.png 1.5x,
                //uploadkimedia.org/wikipedia/commons/thumb/3/34/Microsoft_Office_Excel_%282019%E2%80%93present%29.svgx-Microsoft_Officecel_%282019%E2%80%93present%29.svg.png 2x
              "          data-file-width="512"
              d-file-height="476"
          /></a/    </td>
       /tr>   tr>
        <tdlspan="2" class="infobox-image">
          <ref="/wiki/File:Microsoft_Excel.png" class="image"
           img
           alt="Microsoft Excel.png"
           src="//upload.wikimedia.org/wikipedia/en/thumb/9/94/Microsoft_Excel.png/300px-Microsoft_Excel.png"
           decoding="async"
           width="300"
           height="190"
           srcset="
             //upload.wikimedia.org/wikipedia/en/thumb/9/94/Microsoft_Excel.png/450px-Microsoft_Excel.png 1.5x,
             //upload.wikimedia.org/wikipedia/en/thumb/9/94/Microsoft_Excel.png/600px-Microsoft_Excel.png 2x
           "
           data-file-width="800"
           data-file-height="507"
          /a>
          < class="infobox-caption">
           simple
            href="/wiki/Line_chart" title="Line chart">line chart</a> being
            created in Excel, running on
            <a href="/wiki/Windows_10" title="Windows 10">Windows 10</a>
           /div>
           d>
         </
         <t//       <th scope="row" class="infobox-label" style="white-space: nowrap">
           a href="/wiki/Programmer" title="Programmer">Developer(s)</a>
           h>
            class="infobox-data">
           a href="/wiki/Microsoft" title="Microsoft">Microsoft</a>
           d>
         </
         <t//       <th scope="row" class="infobox-label" style="white-space: nowrap">
           nitial release
           h>
            class="infobox-data">
           987<span class="noprint">; 34&nbsp;years ago</span
           <span style="display: none"
            >&nbsp;(<span class="bday dtstart published updated">1987</span
            >)</span

           d>
         </
         <ttyle="display: none">
            colspan="2" class="infobox-full-data"></td>
         </
         <t//       <th scope="row" class="infobox-label" style="white-space: nowrap">
           a
               href="/wiki/Software_release_life_cycle"
               title="Software release life cycle"
               >Stable release</a
             >
           </th>
           <td class="infobox-data">
             <div style="margin: 0px">
               2103 (16.0.13901.20400) / April&nbsp;13, 2021<span class="noprint"
                 >; 4 months ago</span
               ><span style="display: none"
                 >&nbsp;(<span class="bday dtstart published updated"
                   >2021-04-13</span
                 >)</span
               ><sup id="cite_ref-1" class="reference"
                 ><a href="&#35;cite_note-1">[1]</a></sup
               >
             </div>
           </td>
         </tr>
         <tr style="display: none">
           <td colspan="2"></td>
         </tr>
         <tr>
           <th scope="row" class="infobox-label" style="white-space: nowrap">
             <a href="/wiki/Operating_system" title="Operating system"
               >Operating system</a
             >
           </th>
           <td class="infobox-data">
             <a href="/wiki/Microsoft_Windows" title="Microsoft Windows"
               >Microsoft Windows</a
             >
           </td>
         </tr>
         <tr>
           <th scope="row" class="infobox-label" style="white-space: nowrap">
             <a
               href="/wiki/Software_categories#Categorization_approaches"
               title="Software categories"
               >Type</a
             >
           </th>
           <td class="infobox-data">
             <a href="/wiki/Spreadsheet" title="Spreadsheet">Spreadsheet</a>
           </td>
         </tr>
         <tr>
           <th scope="row" class="infobox-label" style="white-space: nowrap">
             <a href="/wiki/Software_license" title="Software license">License</a>
           </th>
           <td class="infobox-data">
             <a href="/wiki/Trialware" class="mw-redirect" title="Trialware"
               >Trialware</a
             ><sup id="cite_ref-2" class="reference"
               ><a href="&#35;cite_note-2">[2]</a></sup
             >
           </td>
         </tr>
         <tr>
           <th scope="row" class="infobox-label" style="white-space: nowrap">
             Website
           </th>
           <td class="infobox-data">
             <span class="url"
               ><a
                 rel="nofollow"
                 class="external text"
                 href="http://products.office.com/en-us/excel"
                 >products<wbr />.office<wbr />.com<wbr />/en-us<wbr />/excel</a
               ></span
             >
           </td>
         </tr>
       </tbody>
     </table>
     "#;
    */

    #[test]
    fn test_find_first_none() {
        assert_eq!(None, WebTable::find_first(""));
        assert_eq!(None, WebTable::find_first("foo"));
        assert_eq!(None, WebTable::find_first(HTML_NO_TABLE));
    }

    #[test]
    fn test_find_first_empty() {
        let empty = WebTable {
            headers: HashMap::new(),
            data: Vec::new(),
        };
        assert_eq!(Some(empty), WebTable::find_first(TABLE_EMPTY));
    }

    #[test]
    fn test_find_first_some() {
        assert!(WebTable::find_first(TABLE_TH).is_some());
        assert!(WebTable::find_first(TABLE_TD).is_some());
    }

    #[test]
    fn test_find_by_id_none() {
        assert_eq!(None, WebTable::find_by_id("", ""));
        assert_eq!(None, WebTable::find_by_id("foo", "id"));
        assert_eq!(None, WebTable::find_by_id(HTML_NO_TABLE, "id"));
        assert_eq!(None, WebTable::find_by_id(TABLE_EMPTY, "id"));
        assert_eq!(None, WebTable::find_by_id(TABLE_TH, "id"));
        assert_eq!(None, WebTable::find_by_id(TABLE_TH, ""));
        assert_eq!(None, WebTable::find_by_id(HTML_TWO_TABLES, "id"));
    }

    #[test]
    fn test_find_by_id_some() {
        assert!(WebTable::find_by_id(HTML_TWO_TABLES, "first").is_some());
        assert!(WebTable::find_by_id(HTML_TWO_TABLES, "second").is_some());
    }

    #[test]
    fn test_find_by_headers_empty() {
        let headers: [&str; 0] = [];

        assert_eq!(None, WebTable::find_by_headers("", &headers));
        assert_eq!(None, WebTable::find_by_headers("foo", &headers));
        assert_eq!(None, WebTable::find_by_headers(HTML_NO_TABLE, &headers));

        assert!(WebTable::find_by_headers(TABLE_EMPTY, &headers).is_some());
        assert!(WebTable::find_by_headers(HTML_TWO_TABLES, &headers).is_some());
    }

    #[test]
    fn test_find_by_headers_none() {
        let headers = ["Name", "Age"];
        let bad_headers = ["Name", "BAD"];

        assert_eq!(None, WebTable::find_by_headers("", &headers));
        assert_eq!(None, WebTable::find_by_headers("foo", &headers));
        assert_eq!(None, WebTable::find_by_headers(HTML_NO_TABLE, &headers));

        assert_eq!(None, WebTable::find_by_headers(TABLE_EMPTY, &bad_headers));
        assert_eq!(None, WebTable::find_by_headers(TABLE_TH, &bad_headers));

        assert_eq!(None, WebTable::find_by_headers(TABLE_TD, &headers));
        assert_eq!(None, WebTable::find_by_headers(TABLE_TD, &bad_headers));
    }

    #[test]
    fn test_find_by_headers_some() {
        let headers: [&str; 0] = [];
        assert!(WebTable::find_by_headers(TABLE_TH, &headers).is_some());
        assert!(WebTable::find_by_headers(TABLE_TH_TD, &headers).is_some());
        assert!(WebTable::find_by_headers(HTML_TWO_TABLES, &headers).is_some());

        let headers = ["Name"];
        assert!(WebTable::find_by_headers(TABLE_TH, &headers).is_some());
        assert!(WebTable::find_by_headers(TABLE_TH_TD, &headers).is_some());
        assert!(WebTable::find_by_headers(HTML_TWO_TABLES, &headers).is_some());

        let headers = ["Age", "Name"];
        assert!(WebTable::find_by_headers(TABLE_TH, &headers).is_some());
        assert!(WebTable::find_by_headers(TABLE_TH_TD, &headers).is_some());
        assert!(WebTable::find_by_headers(HTML_TWO_TABLES, &headers).is_some());
    }

    #[test]
    fn test_find_first_incomplete_fragment() {
        assert!(WebTable::find_first(HTML_TABLE_FRAGMENT).is_some());
    }

    #[test]
    fn test_headers_empty() {
        let empty = HashMap::new();
        assert_eq!(&empty, WebTable::find_first(TABLE_TD).unwrap().headers());
        assert_eq!(&empty, WebTable::find_first(TABLE_TD_TD).unwrap().headers());
    }

    #[test]
    fn test_headers_nonempty() {
        let mut headers = HashMap::new();
        headers.insert("Name".to_string(), 0);
        headers.insert("Age".to_string(), 1);

        assert_eq!(&headers, WebTable::find_first(TABLE_TH).unwrap().headers());
        assert_eq!(
            &headers,
            WebTable::find_first(TABLE_TH_TD).unwrap().headers()
        );
        assert_eq!(
            &headers,
            WebTable::find_first(TABLE_TH_TH).unwrap().headers()
        );

        headers.insert("Extra".to_string(), 2);
        assert_eq!(
            &headers,
            WebTable::find_first(TABLE_COMPLEX).unwrap().headers()
        );
    }

    #[test]
    fn test_iter_empty() {
        assert_eq!(0, WebTable::find_first(TABLE_EMPTY).unwrap().iter().count());
        assert_eq!(0, WebTable::find_first(TABLE_TH).unwrap().iter().count());
    }

    #[test]
    fn test_iter_nonempty() {
        assert_eq!(1, WebTable::find_first(TABLE_TD).unwrap().iter().count());
        assert_eq!(1, WebTable::find_first(TABLE_TH_TD).unwrap().iter().count());
        assert_eq!(2, WebTable::find_first(TABLE_TD_TD).unwrap().iter().count());
        assert_eq!(1, WebTable::find_first(TABLE_TH_TH).unwrap().iter().count());
        assert_eq!(
            4,
            WebTable::find_first(TABLE_COMPLEX).unwrap().iter().count()
        );
    }

    #[test]
    fn test_row_is_empty() {
        let table = WebTable::find_first(TABLE_TD).unwrap();
        assert_eq!(
            vec![false],
            table.iter().map(|r| r.is_empty()).collect::<Vec<_>>()
        );

        let table = WebTable::find_first(TABLE_COMPLEX).unwrap();
        assert_eq!(
            vec![false, false, true, false],
            table.iter().map(|r| r.is_empty()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_row_len() {
        let table = WebTable::find_first(TABLE_TD).unwrap();
        assert_eq!(vec![2], table.iter().map(|r| r.len()).collect::<Vec<_>>());

        let table = WebTable::find_first(TABLE_COMPLEX).unwrap();
        assert_eq!(
            vec![2, 3, 0, 4],
            table.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_row_len_two_tables() {
        let tables = WebTable::find_all_tables(HTML_TWO_TABLES).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        assert_eq!(vec![2], table_1.iter().map(|r| r.len()).collect::<Vec<_>>());
        assert_eq!(vec![2], table_2.iter().map(|r| r.len()).collect::<Vec<_>>());

        let tables = WebTable::find_all_tables(TWO_TABLES_COMPLEX).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        assert_eq!(
            vec![2, 3, 0, 4],
            table_1.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![2, 3, 0, 4],
            table_2.iter().map(|r| r.len()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_row_get_without_headers() {
        let table = WebTable::find_first(TABLE_TD).unwrap();
        let mut iter = table.iter();
        let row = iter.next().unwrap();

        assert_eq!(None, row.get(""));
        assert_eq!(None, row.get("foo"));
        assert_eq!(None, row.get("Name"));
        assert_eq!(None, row.get("Age"));

        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_get_with_headers() {
        let table = WebTable::find_first(TABLE_TH_TD).unwrap();
        let mut iter = table.iter();
        let row = iter.next().unwrap();

        assert_eq!(None, row.get(""));
        assert_eq!(None, row.get("foo"));
        assert_eq!(Some("John"), row.get("Name"));
        assert_eq!(Some("20"), row.get("Age"));

        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_get_complex() {
        let table = WebTable::find_first(TABLE_COMPLEX).unwrap();
        let mut iter = table.iter();

        let row = iter.next().unwrap();
        assert_eq!(Some("John"), row.get("Name"));
        assert_eq!(Some("20"), row.get("Age"));
        assert_eq!(None, row.get("Extra"));

        let row = iter.next().unwrap();
        assert_eq!(Some("May"), row.get("Name"));
        assert_eq!(Some("30"), row.get("Age"));
        assert_eq!(Some("foo"), row.get("Extra"));

        let row = iter.next().unwrap();
        assert_eq!(None, row.get("Name"));
        assert_eq!(None, row.get("Age"));
        assert_eq!(None, row.get("Extra"));

        let row = iter.next().unwrap();
        assert_eq!(Some("a"), row.get("Name"));
        assert_eq!(Some("b"), row.get("Age"));
        assert_eq!(Some("c"), row.get("Extra"));

        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_two_tables_row_get_complex() {
        let tables = WebTable::find_all_tables(TWO_TABLES_COMPLEX).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        let mut iter_1 = table_1.iter();
        let mut iter_2 = table_2.iter();

        let row_table_1 = iter_1.next().unwrap();
        let row_table_2 = iter_2.next().unwrap();
        assert_eq!(Some("John"), row_table_1.get("Name"));
        assert_eq!(Some("20"), row_table_1.get("Age"));
        assert_eq!(None, row_table_1.get("Extra"));
        assert_eq!(Some("Carpenter"), row_table_2.get("Profession"));
        assert_eq!(Some("Single"), row_table_2.get("Civil State"));
        assert_eq!(None, row_table_2.get("Extra"));

        let row_table_1 = iter_1.next().unwrap();
        let row_table_2 = iter_2.next().unwrap();
        assert_eq!(Some("May"), row_table_1.get("Name"));
        assert_eq!(Some("30"), row_table_1.get("Age"));
        assert_eq!(Some("foo"), row_table_1.get("Extra"));
        assert_eq!(Some("Mechanic"), row_table_2.get("Profession"));
        assert_eq!(Some("Married"), row_table_2.get("Civil State"));
        assert_eq!(Some("bar"), row_table_2.get("Extra"));

        let row_table_1 = iter_1.next().unwrap();
        let row_table_2 = iter_2.next().unwrap();
        assert_eq!(None, row_table_1.get("Name"));
        assert_eq!(None, row_table_1.get("Age"));
        assert_eq!(None, row_table_1.get("Extra"));
        assert_eq!(None, row_table_2.get("Name"));
        assert_eq!(None, row_table_2.get("Age"));
        assert_eq!(None, row_table_2.get("Extra"));

        let row_table_1 = iter_1.next().unwrap();
        let row_table_2 = iter_2.next().unwrap();
        assert_eq!(Some("a"), row_table_1.get("Name"));
        assert_eq!(Some("b"), row_table_1.get("Age"));
        assert_eq!(Some("c"), row_table_1.get("Extra"));
        assert_eq!(Some("e"), row_table_2.get("Profession"));
        assert_eq!(Some("f"), row_table_2.get("Civil State"));
        assert_eq!(Some("g"), row_table_2.get("Extra"));

        assert_eq!(None, iter_1.next());
        assert_eq!(None, iter_2.next());
    }

    #[test]
    fn test_row_as_slice_without_headers() {
        let table = WebTable::find_first(TABLE_TD).unwrap();
        let mut iter = table.iter();

        assert_eq!(&["Name", "Age"], iter.next().unwrap().as_slice());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_as_slice_without_headers_two_tables() {
        let tables = WebTable::find_all_tables(TWO_TABLES_TD).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        let mut iter_1 = table_1.iter();
        let mut iter_2 = table_2.iter();

        assert_eq!(&["Name", "Age"], iter_1.next().unwrap().as_slice());
        assert_eq!(
            &["Profession", "Civil State"],
            iter_2.next().unwrap().as_slice()
        );
        assert_eq!(None, iter_1.next());
        assert_eq!(None, iter_2.next());
    }

    #[test]
    fn test_row_as_slice_with_headers() {
        let table = WebTable::find_first(TABLE_TH_TD).unwrap();
        let mut iter = table.iter();

        assert_eq!(&["John", "20"], iter.next().unwrap().as_slice());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_as_slice_with_headers_two_tables() {
        let tables = WebTable::find_all_tables(TWO_TABLES_TH_TD).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        let mut iter_1 = table_1.iter();
        let mut iter_2 = table_2.iter();

        assert_eq!(&["John", "20"], iter_1.next().unwrap().as_slice());
        assert_eq!(&["Mechanic", "Single"], iter_2.next().unwrap().as_slice());
        assert_eq!(None, iter_1.next());
        assert_eq!(None, iter_2.next());
    }

    #[test]
    fn test_row_as_slice_complex() {
        let table = WebTable::find_first(TABLE_COMPLEX).unwrap();
        let mut iter = table.iter();
        let empty: [&str; 0] = [];

        assert_eq!(&["John", "20"], iter.next().unwrap().as_slice());
        assert_eq!(&["May", "30", "foo"], iter.next().unwrap().as_slice());
        assert_eq!(&empty, iter.next().unwrap().as_slice());
        assert_eq!(&["a", "b", "c", "d"], iter.next().unwrap().as_slice());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_as_slice_complex_two_tables() {
        let tables = WebTable::find_all_tables(TWO_TABLES_COMPLEX).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        let mut iter_1 = table_1.iter();
        let mut iter_2 = table_2.iter();
        let empty: [&str; 0] = [];

        assert_eq!(&["John", "20"], iter_1.next().unwrap().as_slice());
        assert_eq!(&["May", "30", "foo"], iter_1.next().unwrap().as_slice());
        assert_eq!(&empty, iter_1.next().unwrap().as_slice());
        assert_eq!(&["a", "b", "c", "d"], iter_1.next().unwrap().as_slice());
        assert_eq!(None, iter_1.next());
        assert_eq!(&["Carpenter", "Single"], iter_2.next().unwrap().as_slice());
        assert_eq!(
            &["Mechanic", "Married", "bar"],
            iter_2.next().unwrap().as_slice()
        );
        assert_eq!(&empty, iter_2.next().unwrap().as_slice());
        assert_eq!(&["e", "f", "g", "h"], iter_2.next().unwrap().as_slice());
        assert_eq!(None, iter_2.next());
    }

    #[test]
    fn test_row_iter_simple() {
        let table = WebTable::find_first(TABLE_TD).unwrap();
        let row = table.iter().next().unwrap();
        let mut iter = row.iter();

        assert_eq!(Some("Name"), iter.next().map(String::as_str));
        assert_eq!(Some("Age"), iter.next().map(String::as_str));
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_iter_simple_two_tables() {
        let tables = WebTable::find_all_tables(TWO_TABLES_TD).unwrap();
        let mut tables_iter = tables.iter();
        let table_1 = tables_iter.next().unwrap();
        let table_2 = tables_iter.next().unwrap();
        let row_1 = table_1.iter().next().unwrap();
        let row_2 = table_2.iter().next().unwrap();
        let mut iter_1 = row_1.iter();
        let mut iter_2 = row_2.iter();

        assert_eq!(Some("Name"), iter_1.next().map(String::as_str));
        assert_eq!(Some("Age"), iter_1.next().map(String::as_str));
        assert_eq!(None, iter_1.next());
        assert_eq!(Some("Profession"), iter_2.next().map(String::as_str));
        assert_eq!(Some("Civil State"), iter_2.next().map(String::as_str));
        assert_eq!(None, iter_2.next());
    }

    #[test]
    fn test_row_iter_complex() {
        let table = WebTable::find_first(TABLE_COMPLEX).unwrap();
        let mut table_iter = table.iter();

        let row = table_iter.next().unwrap();
        let mut iter = row.iter();
        assert_eq!(Some("John"), iter.next().map(String::as_str));
        assert_eq!(Some("20"), iter.next().map(String::as_str));
        assert_eq!(None, iter.next());

        let row = table_iter.next().unwrap();
        let mut iter = row.iter();
        assert_eq!(Some("May"), iter.next().map(String::as_str));
        assert_eq!(Some("30"), iter.next().map(String::as_str));
        assert_eq!(Some("foo"), iter.next().map(String::as_str));
        assert_eq!(None, iter.next());

        let row = table_iter.next().unwrap();
        let mut iter = row.iter();
        assert_eq!(None, iter.next());

        let row = table_iter.next().unwrap();
        let mut iter = row.iter();
        assert_eq!(Some("a"), iter.next().map(String::as_str));
        assert_eq!(Some("b"), iter.next().map(String::as_str));
        assert_eq!(Some("c"), iter.next().map(String::as_str));
        assert_eq!(Some("d"), iter.next().map(String::as_str));
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_row_iter_complex_two_tables() {
        let tables = WebTable::find_all_tables(TWO_TABLES_COMPLEX).unwrap();
        let mut tables_iter = tables.iter();
        let mut table_1 = tables_iter.next().unwrap().iter();
        let mut table_2 = tables_iter.next().unwrap().iter();

        let row_1 = table_1.next().unwrap();
        let row_2 = table_2.next().unwrap();
        let mut iter_1 = row_1.iter();
        let mut iter_2 = row_2.iter();
        assert_eq!(Some("John"), iter_1.next().map(String::as_str));
        assert_eq!(Some("20"), iter_1.next().map(String::as_str));
        assert_eq!(None, iter_1.next());
        assert_eq!(Some("Carpenter"), iter_2.next().map(String::as_str));
        assert_eq!(Some("Single"), iter_2.next().map(String::as_str));
        assert_eq!(None, iter_2.next());

        let row_1 = table_1.next().unwrap();
        let row_2 = table_2.next().unwrap();
        let mut iter_1 = row_1.iter();
        let mut iter_2 = row_2.iter();
        assert_eq!(Some("May"), iter_1.next().map(String::as_str));
        assert_eq!(Some("30"), iter_1.next().map(String::as_str));
        assert_eq!(Some("foo"), iter_1.next().map(String::as_str));
        assert_eq!(None, iter_1.next());
        assert_eq!(Some("Mechanic"), iter_2.next().map(String::as_str));
        assert_eq!(Some("Married"), iter_2.next().map(String::as_str));
        assert_eq!(Some("bar"), iter_2.next().map(String::as_str));
        assert_eq!(None, iter_2.next());

        let row_1 = table_1.next().unwrap();
        let row_2 = table_2.next().unwrap();
        let mut iter_1 = row_1.iter();
        let mut iter_2 = row_2.iter();
        assert_eq!(None, iter_1.next());
        assert_eq!(None, iter_2.next());

        let row_1 = table_1.next().unwrap();
        let row_2 = table_2.next().unwrap();
        let mut iter_1 = row_1.iter();
        let mut iter_2 = row_2.iter();
        assert_eq!(Some("a"), iter_1.next().map(String::as_str));
        assert_eq!(Some("b"), iter_1.next().map(String::as_str));
        assert_eq!(Some("c"), iter_1.next().map(String::as_str));
        assert_eq!(Some("d"), iter_1.next().map(String::as_str));
        assert_eq!(None, iter_1.next());
        assert_eq!(Some("e"), iter_2.next().map(String::as_str));
        assert_eq!(Some("f"), iter_2.next().map(String::as_str));
        assert_eq!(Some("g"), iter_2.next().map(String::as_str));
        assert_eq!(Some("h"), iter_2.next().map(String::as_str));
        assert_eq!(None, iter_2.next());
    }
}
/*
    #[test]
    fn test_wikipedia_swapped_rows_columns() {
        // empty columns
        let cols = nu_protocol::value::Value {
            value: nu_protocol::UntaggedValue::Primitive(nu_protocol::Primitive::String(
                "".to_string(),
            )),
            tag: nu_source::Tag::unknown(),
        };

        // this table is taken straight from wikipedia with no changes
        let table = retrieve_tables(HTML_TABLE_WIKIPEDIA_COLUMNS_AS_ROWS, &cols, true);

        let expected = vec![UntaggedValue::row(indexmap! {
                 "Stable release".to_string() => UntaggedValue::string("\n          2103 (16.0.13901.20400) / April\u{a0}13, 2021; 4 months ago\u{a0}(2021-04-13)[1]\n        ").into(),
                 "Developer(s)".to_string() => UntaggedValue::string("Microsoft").into(),
                 "Operating system".to_string() => UntaggedValue::string("Microsoft Windows").into(),
                 "Type".to_string() => UntaggedValue::string("Spreadsheet").into(),
                 "License".to_string() => UntaggedValue::string("Trialware[2]").into(),
                 "".to_string() => UntaggedValue::string("").into(),
                 "Website".to_string() => UntaggedValue::string("products.office.com/en-us/excel").into(),
                 "Initial release".to_string() => UntaggedValue::string("1987; 34\u{a0}years ago\u{a0}(1987)").into(),
             }).into()];

        assert_eq!(table, expected);
    }

    #[test]
    fn test_wikipedia_table_with_column_headers() {
        let cols = UntaggedValue::table(&[
            UntaggedValue::string("Format".to_string()).into(),
            UntaggedValue::string("Extension".to_string()).into(),
            UntaggedValue::string("Description".to_string()).into(),
        ])
        .into();

        // this table is taken straight from wikipedia with no changes
        let table = retrieve_tables(HTML_TABLE_WIKIPEDIA_WITH_COLUMN_NAMES, &cols, true);
        let expected = vec![
                 UntaggedValue::row(indexmap! {
                     "Format".to_string() => UntaggedValue::string("Excel Workbook").into(),
                     "Extension".to_string() => UntaggedValue::string(".xlsx").into(),
                     "Description".to_string() => UntaggedValue::string("The default Excel 2007 and later workbook format. In reality, a Zip compressed archive with a directory structure of XML text documents. Functions as the primary

    //replacement for the former binary .xls format, although it does not support Excel macros for security reasons. Saving as .xlsx offers file size reduction over .xls[38]").into(),
                 }).into(),
                 UntaggedValue::row(indexmap! {
                     "Format".to_string() => UntaggedValue::string("Excel Macro-enabled Workbook").into(),
                     "Extension".to_string() => UntaggedValue::string(".xlsm").into(),
                     "Description".to_string() => UntaggedValue::string("As Excel Workbook, but with macro support.").into(),
                 }).into(),
                 UntaggedValue::row(indexmap! {
                     "Format".to_string() => UntaggedValue::string("Excel Binary Workbook").into(),
                     "Extension".to_string() => UntaggedValue::string(".xlsb").into(),
                     "Description".to_string() => UntaggedValue::string("As Excel Macro-enabled Workbook, but storing information in binary form rather than XML documents for opening and saving documents more quickly and efficiently. Intended especially for very large documents with tens of thousands of rows, and/or several hundreds of columns. This format is very useful for shrinking large Excel files as is often the case when doing data analysis.").into(),
                 }).into(),
                 UntaggedValue::row(indexmap! {
                     "Format".to_string() => UntaggedValue::string("Excel Macro-enabled Template").into(),
                     "Extension".to_string() => UntaggedValue::string(".xltm").into(),
                     "Description".to_string() => UntaggedValue::string("A template document that forms a basis for actual workbooks, with macro support. The replacement for the old .xlt format.").into(),
                 }).into(),
                 UntaggedValue::row(indexmap! {
                     "Format".to_string() => UntaggedValue::string("Excel Add-in").into(),
                     "Extension".to_string() => UntaggedValue::string(".xlam").into(),
                     "Description".to_string() => UntaggedValue::string("Excel add-in to add extra functionality and tools. Inherent macro support because of the file purpose.").into(),
                 }).into(),
             ];

        assert_eq!(table, expected);
    }
*/
