pub trait Host {
    fn stdout(&mut self, out: &str);
    fn stderr(&mut self, out: &str);
}

impl Host for Box<dyn Host> {
    fn stdout(&mut self, out: &str) {
        (**self).stdout(out)
    }

    fn stderr(&mut self, out: &str) {
        (**self).stderr(out)
    }
}

crate struct BasicHost;

impl Host for BasicHost {
    fn stdout(&mut self, out: &str) {
        match out {
            "\n" => println!(""),
            other => println!("{}", other),
        }
    }

    fn stderr(&mut self, out: &str) {
        match out {
            "\n" => eprintln!(""),
            other => eprintln!("{}", other),
        }
    }
}
