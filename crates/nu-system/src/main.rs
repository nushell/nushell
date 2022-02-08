use std::time::Duration;

fn main() {
    for proc in nu_system::collect_proc(Duration::from_millis(100), false) {
        // if proc.cpu_usage() > 0.1 {
        println!(
            "{} - {} - {} - {:.1} - {}M - {}M",
            proc.pid(),
            proc.name(),
            proc.status(),
            proc.cpu_usage(),
            proc.mem_size() / (1024 * 1024),
            proc.virtual_size() / (1024 * 1024),
        )
        // }
    }
}
