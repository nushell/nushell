mod query;
mod query_json;
mod query_web;
mod query_webpage_info;
mod query_xml;
mod web_tables;

pub use query::Query;
pub use query_json::{QueryJson, execute_json_query};
pub use query_web::{QueryWeb, parse_selector_params};
pub use query_xml::{QueryXml, execute_xpath_query};
pub use web_tables::WebTable;
