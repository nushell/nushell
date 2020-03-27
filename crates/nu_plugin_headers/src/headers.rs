pub struct Headers;
impl Headers {
    pub fn new() -> Headers {
        Headers
    }
}

impl WholeStreamCommand for Headers {
    fn name(&self) -> &str{
        "headers"
    }
    fn signature(&self) -> Signature {
        Signature::build("headers")
    }
    fn usage(&self) -> &str {
        "Use the first row of the table as headers"
    }
}