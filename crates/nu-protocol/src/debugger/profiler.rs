use crate::ast::PipelineElement;
use crate::debugger::Debugger;
use crate::Span;
use std::time::Instant;

/// Basic profiler
#[derive(Default)]
pub struct Profiler {
    depth: i64,
    max_depth: i64,
    element_start_times: Vec<(Span, Instant)>,
    element_durations_ns: Vec<(Span, u128)>,
}

impl Profiler {
    pub fn report(&self) {
        println!("Profiler report:\n===============");
        for (span, duration_ns) in &self.element_durations_ns {
            let dur_us = duration_ns / 1000;
            println!("span: {span:?}, duration: {dur_us} us");
        }
    }
}

impl Debugger for Profiler {
    fn enter_block(&mut self) {
        self.depth += 1;
    }

    fn leave_block(&mut self) {
        self.depth -= 1;
    }

    fn enter_element(&mut self, element: &PipelineElement) {
        self.element_start_times
            .push((element.span(), Instant::now()));
    }

    fn leave_element(&mut self, element: &PipelineElement) {
        let Some((span, start)) = self.element_start_times.pop() else {
            eprintln!(
                "Error: Profiler left pipeline element without matching element start time stamp."
            );
            return;
        };

        self.element_durations_ns
            .push((span, start.elapsed().as_nanos()));
    }
}
