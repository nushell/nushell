mod query;
mod query_json;
mod query_web;
mod query_xml;
mod web_tables;

pub use query::Query;
pub use query_json::{execute_json_query, QueryJson};
pub use query_web::{parse_selector_params, QueryWeb};
pub use query_xml::{execute_xpath_query, QueryXml};
pub use web_tables::WebTable;
