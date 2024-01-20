use crate::debugger::Debugger;
use std::time::Instant;

/// Basic profiler
#[derive(Default)]
pub struct Profiler {
    pub instants: Vec<Instant>,
    pub durations_us: Vec<u128>,
}

impl Profiler {
    pub fn report(&self) {
        println!("Report ({} durations):", self.durations_us.len());
        println!("=======");
        for duration in &self.durations_us {
            println!("Duration: {duration:5} us");
        }
    }
}

impl Debugger for Profiler {
    fn on_block_enter(&mut self) {
        self.instants.push(Instant::now());
        println!(
            "Entered block with debugger! {} timestamps, {} durations",
            self.instants.len(),
            self.durations_us.len()
        );
    }

    fn on_block_leave(&mut self) {
        let start = self.instants.pop().unwrap();
        self.durations_us.push(start.elapsed().as_micros());
        println!(
            "Left block with debugger! {} timestamps, {} durations",
            self.instants.len(),
            self.durations_us.len()
        );
    }
}
