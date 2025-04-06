pub trait DbRow: std::marker::Send {
    fn fields(&self) -> Vec<String>;
    fn values(&self) -> Vec<String>;
}
