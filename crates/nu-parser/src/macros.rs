#[macro_export]
macro_rules! return_ok {
    ($expr:expr) => {
        match $expr {
            Ok(val) => return Ok(val),
            Err(_) => {}
        }
    };
}
