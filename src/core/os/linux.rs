use std::fs::File;
use std::io::{BufRead, BufReader};

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct LinuxCpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

#[allow(dead_code)]
impl LinuxCpuTimes {
    pub fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    pub fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }
}

#[allow(dead_code)]
pub fn read_proc_stat() -> Option<(LinuxCpuTimes, Vec<LinuxCpuTimes>)> {
    let file = File::open("/proc/stat").ok()?;
    let reader = BufReader::new(file);

    let mut global_times = None;
    let mut core_times = Vec::new();

    for line in reader.lines().flatten() {
        if line.starts_with("cpu ") {
            global_times = parse_cpu_line(&line[4..]);
        } else if line.starts_with("cpu") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let rest = line[parts[0].len()..].trim_start();
                if let Some(times) = parse_cpu_line(rest) {
                    core_times.push(times);
                }
            }
        }
    }

    global_times.map(|gt| (gt, core_times))
}

#[allow(dead_code)]
fn parse_cpu_line(line: &str) -> Option<LinuxCpuTimes> {
    let mut iter = line.split_whitespace();
    let user = iter.next()?.parse().ok()?;
    let nice = iter.next()?.parse().ok()?;
    let system = iter.next()?.parse().ok()?;
    let idle = iter.next()?.parse().ok()?;
    let iowait = iter.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let irq = iter.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let softirq = iter.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let steal = iter.next().and_then(|v| v.parse().ok()).unwrap_or(0);

    Some(LinuxCpuTimes {
        user,
        nice,
        system,
        idle,
        iowait,
        irq,
        softirq,
        steal,
    })
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct LinuxMemInfo {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

#[allow(dead_code)]
pub fn read_proc_meminfo() -> Option<LinuxMemInfo> {
    let file = File::open("/proc/meminfo").ok()?;
    let reader = BufReader::new(file);

    let mut info = LinuxMemInfo::default();

    for line in reader.lines().flatten() {
        let mut parts = line.split(':');
        let key = parts.next()?.trim();
        let val_str = parts.next()?.trim();
        let num_str = val_str.split_whitespace().next().unwrap_or("0");
        let num: u64 = num_str.parse().unwrap_or(0);

        match key {
            "MemTotal" => info.total_kb = num,
            "MemFree" => info.free_kb = num,
            "MemAvailable" => info.available_kb = num,
            "Buffers" => info.buffers_kb = num,
            "Cached" => info.cached_kb = num,
            "SwapTotal" => info.swap_total_kb = num,
            "SwapFree" => info.swap_free_kb = num,
            _ => {}
        }
    }

    Some(info)
}
