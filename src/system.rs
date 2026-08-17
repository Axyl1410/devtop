use std::collections::VecDeque;
use sysinfo::{Disks, Networks, Pid, ProcessesToUpdate, System};

pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub status: String,
    pub cmd: String,
}

pub struct DiskItem {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub file_system: String,
}

pub struct SystemStats {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub core_count: usize,
    pub cpu_history: VecDeque<u64>,
    pub memory_history: VecDeque<u64>,
    pub net_rx_history: VecDeque<u64>,
    pub net_tx_history: VecDeque<u64>,
    pub prev_rx: u64,
    pub prev_tx: u64,
    pub current_rx_speed: u64,
    pub current_tx_speed: u64,
}

impl SystemStats {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let os_name = System::name().unwrap_or_else(|| "Linux/Unix".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "".to_string());
        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Generic CPU".to_string());
        let core_count = sys.cpus().len();

        let history_len = 60;
        let mut cpu_history = VecDeque::with_capacity(history_len);
        let mut memory_history = VecDeque::with_capacity(history_len);
        let mut net_rx_history = VecDeque::with_capacity(history_len);
        let mut net_tx_history = VecDeque::with_capacity(history_len);

        for _ in 0..history_len {
            cpu_history.push_back(0);
            memory_history.push_back(0);
            net_rx_history.push_back(0);
            net_tx_history.push_back(0);
        }

        Self {
            sys,
            disks,
            networks,
            hostname,
            os_name,
            os_version,
            kernel_version,
            cpu_model,
            core_count,
            cpu_history,
            memory_history,
            net_rx_history,
            net_tx_history,
            prev_rx: 0,
            prev_tx: 0,
            current_rx_speed: 0,
            current_tx_speed: 0,
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.disks.refresh(true);
        self.networks.refresh(true);

        // Update CPU history
        let global_cpu = self.sys.global_cpu_usage() as u64;
        self.cpu_history.pop_front();
        self.cpu_history.push_back(global_cpu.min(100));

        // Update Memory history
        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        let mem_percent = if total_mem > 0 {
            ((used_mem as f64 / total_mem as f64) * 100.0) as u64
        } else {
            0
        };
        self.memory_history.pop_front();
        self.memory_history.push_back(mem_percent.min(100));

        // Network traffic
        let mut total_rx = 0;
        let mut total_tx = 0;
        for (_interface_name, network) in &self.networks {
            total_rx += network.total_received();
            total_tx += network.total_transmitted();
        }

        if self.prev_rx > 0 && total_rx >= self.prev_rx {
            self.current_rx_speed = total_rx - self.prev_rx;
        }
        if self.prev_tx > 0 && total_tx >= self.prev_tx {
            self.current_tx_speed = total_tx - self.prev_tx;
        }
        self.prev_rx = total_rx;
        self.prev_tx = total_tx;

        self.net_rx_history.pop_front();
        self.net_rx_history.push_back(self.current_rx_speed / 1024); // KB/s

        self.net_tx_history.pop_front();
        self.net_tx_history.push_back(self.current_tx_speed / 1024); // KB/s
    }

    pub fn get_processes(&self) -> Vec<ProcessItem> {
        let total_mem = self.sys.total_memory() as f32;
        self.sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let mem = proc.memory();
                let mem_pct = if total_mem > 0.0 {
                    (mem as f32 / total_mem) * 100.0
                } else {
                    0.0
                };
                let cmd_str = proc
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" ");

                ProcessItem {
                    pid: pid.as_u32(),
                    name: proc.name().to_string_lossy().into_owned(),
                    cpu_usage: proc.cpu_usage(),
                    memory_bytes: mem,
                    memory_percent: mem_pct,
                    status: format!("{:?}", proc.status()),
                    cmd: if cmd_str.is_empty() {
                        proc.name().to_string_lossy().into_owned()
                    } else {
                        cmd_str
                    },
                }
            })
            .collect()
    }

    pub fn get_disks(&self) -> Vec<DiskItem> {
        self.disks
            .iter()
            .map(|disk| DiskItem {
                name: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                file_system: disk.file_system().to_string_lossy().into_owned(),
            })
            .collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> bool {
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            process.kill()
        } else {
            false
        }
    }
}
