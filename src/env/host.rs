pub trait Host {
    fn stdout(&mut self, out: &str);
}

crate struct BasicHost;

impl Host for BasicHost {
    fn stdout(&mut self, out: &str) {
        println!("{}", out)
    }
}
