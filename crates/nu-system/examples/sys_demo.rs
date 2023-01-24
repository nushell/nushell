fn main() {
    #[cfg(any(
        target_os = "android",
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    ))]
    {
        let cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        for run in 1..=10 {
            for proc in nu_system::collect_proc(std::time::Duration::from_millis(100), false) {
                if proc.cpu_usage() > 0.00001 {
                    println!(
                        "{} - {} - {} - {} - {:.2}% - {}M - {}M - {} procs",
                        run,
                        proc.pid(),
                        proc.name(),
                        proc.status(),
                        proc.cpu_usage() / cores as f64,
                        proc.mem_size() / (1024 * 1024),
                        proc.virtual_size() / (1024 * 1024),
                        cores,
                    )
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }
}
