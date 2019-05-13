pub trait Host {
    fn stdout(&mut self, out: &str);
}

impl Host for Box<dyn Host> {
    fn stdout(&mut self, out: &str) {
        (**self).stdout(out)
    }
}

crate struct BasicHost;

impl Host for BasicHost {
    fn stdout(&mut self, out: &str) {
        println!("{}", out)
    }
}
