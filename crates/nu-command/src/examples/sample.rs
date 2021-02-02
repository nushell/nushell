use nu_protocol::{row, Value};
use nu_test_support::value::{date, int, string};

pub mod ls {
    use super::*;

    pub fn file_listing() -> Vec<Value> {
        vec![
            row! {
                   "name".to_string() => string("Andres.txt"),
                   "type".to_string() =>       string("File"),
               "chickens".to_string() =>              int(10),
               "modified".to_string() =>   date("2019-07-23")
            },
            row! {
                   "name".to_string() =>   string("Jonathan"),
                   "type".to_string() =>        string("Dir"),
               "chickens".to_string() =>               int(5),
               "modified".to_string() =>   date("2019-07-23")
            },
            row! {
                   "name".to_string() =>  string("Darren.txt"),
                   "type".to_string() =>        string("File"),
               "chickens".to_string() =>               int(20),
               "modified".to_string() =>    date("2019-09-24")
            },
            row! {
                   "name".to_string() =>      string("Yehuda"),
                   "type".to_string() =>         string("Dir"),
               "chickens".to_string() =>                int(4),
               "modified".to_string() =>    date("2019-09-24")
            },
        ]
    }
}
