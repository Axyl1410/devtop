pub mod cpu;
pub mod disks;
pub mod memory;
pub mod network;
pub mod os;
pub mod ports;
pub mod process;
pub mod signals;

pub use cpu::{CpuHarvest, CpuTracker};
pub use disks::DiskHarvest;
pub use memory::{MemoryHarvest, MemoryTracker};
pub use network::{InterfaceHarvest, NetworkHarvest, NetworkTracker};
pub use ports::{PortBinding, scan_listening_ports};
pub use process::{ProcessHarvest, ProcessTreeItem, build_process_tree};
pub use signals::{ProcessSignal, send_signal_to_pid};

use os::GenericOsEngine;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use sysinfo::System;

#[allow(dead_code)]
pub struct SystemCore {
    os_engine: GenericOsEngine,
    pub cpu_tracker: CpuTracker,
    pub memory_tracker: MemoryTracker,
    pub network_tracker: NetworkTracker,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub core_count: usize,
    pub cached_ports: Vec<PortBinding>,
    last_ports_refresh: Instant,
    last_refresh_time: Instant,
}

impl SystemCore {
    pub fn new() -> Self {
        let mut os_engine = GenericOsEngine::new();
        os_engine.refresh();

        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
        let os_name = System::name().unwrap_or_else(|| "Linux/Unix".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "".to_string());
        let cpu_model = os_engine
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Generic CPU".to_string());
        let core_count = os_engine.sys.cpus().len();

        let history_len = 60;
        let mut cpu_tracker = CpuTracker::new(history_len);
        let mut memory_tracker = MemoryTracker::new(history_len);
        let mut network_tracker = NetworkTracker::new(history_len);

        // Initial seed of trackers
        let global_cpu = os_engine.sys.global_cpu_usage();
        let per_core: Vec<f32> = os_engine.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        cpu_tracker.update(global_cpu, &per_core);

        let total_mem = os_engine.sys.total_memory();
        let used_mem = os_engine.sys.used_memory();
        let free_mem = os_engine.sys.free_memory();
        let avail_mem = os_engine.sys.available_memory();
        let cache_mem = avail_mem.saturating_sub(free_mem);

        let mem_pct = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        let cache_pct = if total_mem > 0 {
            (cache_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        let total_swap = os_engine.sys.total_swap();
        let used_swap = os_engine.sys.used_swap();
        let swap_pct = if total_swap > 0 {
            (used_swap as f32 / total_swap as f32) * 100.0
        } else {
            0.0
        };
        memory_tracker.update(
            used_mem, used_swap, cache_mem, total_mem, total_swap, mem_pct, swap_pct, cache_pct,
        );

        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        let mut init_ifaces: Vec<(String, u64, u64)> = Vec::new();
        for (iface_name, net) in &os_engine.networks {
            let rx = net.total_received();
            let tx = net.total_transmitted();
            total_rx += rx;
            total_tx += tx;
            init_ifaces.push((iface_name.to_string(), rx, tx));
        }
        network_tracker.update(total_rx, total_tx, 1.0, &init_ifaces);

        let cached_ports = scan_listening_ports();

        Self {
            os_engine,
            cpu_tracker,
            memory_tracker,
            network_tracker,
            hostname,
            os_name,
            os_version,
            kernel_version,
            cpu_model,
            core_count,
            cached_ports,
            last_ports_refresh: Instant::now(),
            last_refresh_time: Instant::now(),
        }
    }

    pub fn refresh(&mut self) {
        let now = Instant::now();
        let delta_secs = now
            .duration_since(self.last_refresh_time)
            .as_secs_f64()
            .max(0.1);
        self.last_refresh_time = now;

        self.os_engine.refresh();

        // 1. Update CPU telemetry
        let global_cpu = self.os_engine.sys.global_cpu_usage();
        let per_core: Vec<f32> = self
            .os_engine
            .sys
            .cpus()
            .iter()
            .map(|c| c.cpu_usage())
            .collect();
        self.cpu_tracker.update(global_cpu, &per_core);

        // 2. Update Memory telemetry
        let total_mem = self.os_engine.sys.total_memory();
        let used_mem = self.os_engine.sys.used_memory();
        let free_mem = self.os_engine.sys.free_memory();
        let avail_mem = self.os_engine.sys.available_memory();
        let cache_mem = avail_mem.saturating_sub(free_mem);

        let mem_pct = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        let cache_pct = if total_mem > 0 {
            (cache_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };
        let total_swap = self.os_engine.sys.total_swap();
        let used_swap = self.os_engine.sys.used_swap();
        let swap_pct = if total_swap > 0 {
            (used_swap as f32 / total_swap as f32) * 100.0
        } else {
            0.0
        };
        self.memory_tracker.update(
            used_mem, used_swap, cache_mem, total_mem, total_swap, mem_pct, swap_pct, cache_pct,
        );

        // 3. Update Network telemetry (global + per-interface)
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        let mut interfaces_raw: Vec<(String, u64, u64)> = Vec::new();
        for (iface_name, net) in &self.os_engine.networks {
            let rx = net.total_received();
            let tx = net.total_transmitted();
            total_rx += rx;
            total_tx += tx;
            interfaces_raw.push((iface_name.to_string(), rx, tx));
        }
        self.network_tracker
            .update(total_rx, total_tx, delta_secs, &interfaces_raw);

        // 4. Update Ports every 2 seconds
        if self.last_ports_refresh.elapsed() >= Duration::from_secs(2) {
            let mut ports = scan_listening_ports();
            // Enrich ports with process info
            let procs = self.get_processes();
            let proc_map: HashMap<u32, ProcessHarvest> =
                procs.into_iter().map(|p| (p.pid, p)).collect();

            for binding in &mut ports {
                if let Some(pid) = binding.pid {
                    if let Some(proc) = proc_map.get(&pid) {
                        binding.process_name = Some(proc.name.clone());
                        binding.cmd = Some(proc.cmd.clone());
                        binding.cwd = Some(proc.cwd.clone());
                        binding.user = Some(proc.user.clone());
                    }
                }
            }

            self.cached_ports = ports;
            self.last_ports_refresh = Instant::now();
        }
    }

    pub fn get_processes(&self) -> Vec<ProcessHarvest> {
        let total_mem = self.os_engine.sys.total_memory() as f32;
        // sysinfo returns cpu_usage() in unnormalized form (can exceed 100% on multi-core).
        // Normalize by dividing by core count → 0-100% of total system CPU, like bottom's default.
        let num_cpus = self.core_count.max(1) as f32;
        let mut parent_to_children: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut pid_to_ports: HashMap<u32, Vec<u16>> = HashMap::new();

        // Map cached ports to PIDs
        for p in &self.cached_ports {
            if let Some(pid) = p.pid {
                pid_to_ports.entry(pid).or_default().push(p.port);
            }
        }

        // Map parent-child hierarchy
        for (pid, proc) in self.os_engine.sys.processes() {
            let pid_u32 = pid.as_u32();
            if let Some(parent_pid) = proc.parent() {
                parent_to_children
                    .entry(parent_pid.as_u32())
                    .or_default()
                    .push(pid_u32);
            }
        }

        self.os_engine
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let pid_u32 = pid.as_u32();
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

                let exe_str = proc
                    .exe()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let cwd_str = proc
                    .cwd()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();

                let user_name = proc
                    .user_id()
                    .and_then(|uid| self.os_engine.users.get_user_by_id(uid))
                    .map(|u| u.name().to_string())
                    .unwrap_or_else(|| {
                        proc.user_id()
                            .map(|u| format!("{:?}", u))
                            .unwrap_or_else(|| "-".to_string())
                    });

                let children = parent_to_children
                    .get(&pid_u32)
                    .cloned()
                    .unwrap_or_default();
                let ports = pid_to_ports.get(&pid_u32).cloned().unwrap_or_default();

                ProcessHarvest {
                    pid: pid_u32,
                    parent_pid: proc.parent().map(|p| p.as_u32()),
                    name: proc.name().to_string_lossy().into_owned(),
                    // Normalize: sysinfo reports 400% for a process using 100% of 4 cores.
                    // Divide by num_cpus to get 0-100% of total system capacity.
                    cpu_usage: (proc.cpu_usage() / num_cpus).min(100.0),
                    memory_bytes: mem,
                    virtual_memory_bytes: proc.virtual_memory(),
                    memory_percent: mem_pct,
                    status: format!("{:?}", proc.status()),
                    cmd: if cmd_str.is_empty() {
                        proc.name().to_string_lossy().into_owned()
                    } else {
                        cmd_str
                    },
                    exe: exe_str,
                    cwd: cwd_str,
                    user: user_name,
                    run_time_secs: proc.run_time(),
                    ports,
                    children,
                }
            })
            .collect()
    }

    pub fn get_process_by_pid(&self, pid: u32) -> Option<ProcessHarvest> {
        self.get_processes().into_iter().find(|p| p.pid == pid)
    }

    pub fn get_ports(&self) -> Vec<PortBinding> {
        self.cached_ports.clone()
    }

    pub fn get_cpu(&self) -> CpuHarvest {
        let global_usage = self.os_engine.sys.global_cpu_usage();
        let per_core_usage: Vec<f32> = self
            .os_engine
            .sys
            .cpus()
            .iter()
            .map(|c| c.cpu_usage())
            .collect();
        let load = System::load_average();

        CpuHarvest {
            model: self.cpu_model.clone(),
            core_count: self.core_count,
            global_usage,
            per_core_usage,
            load_avg: (load.one, load.five, load.fifteen),
        }
    }

    pub fn get_memory(&self) -> MemoryHarvest {
        let total_bytes = self.os_engine.sys.total_memory();
        let used_bytes = self.os_engine.sys.used_memory();
        let free_bytes = self.os_engine.sys.free_memory();
        let available_bytes = self.os_engine.sys.available_memory();
        let cache_bytes = available_bytes.saturating_sub(free_bytes);
        let swap_total_bytes = self.os_engine.sys.total_swap();
        let swap_used_bytes = self.os_engine.sys.used_swap();

        let used_percent = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        let cache_percent = if total_bytes > 0 {
            (cache_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        let swap_used_percent = if swap_total_bytes > 0 {
            (swap_used_bytes as f32 / swap_total_bytes as f32) * 100.0
        } else {
            0.0
        };

        MemoryHarvest {
            total_bytes,
            used_bytes,
            free_bytes,
            available_bytes,
            cache_bytes,
            swap_total_bytes,
            swap_used_bytes,
            used_percent,
            swap_used_percent,
            cache_percent,
        }
    }

    pub fn get_network(&self) -> NetworkHarvest {
        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;

        // Build interface list using pre-computed per-interface speeds from the tracker
        let interfaces: Vec<InterfaceHarvest> = self
            .os_engine
            .networks
            .iter()
            .map(|(name, net)| {
                let rx = net.total_received();
                let tx = net.total_transmitted();
                total_rx_bytes += rx;
                total_tx_bytes += tx;

                // Look up per-interface speed from the tracker's cached per-iface map
                let (iface_rx_speed, iface_tx_speed) = self
                    .network_tracker
                    .prev_iface
                    .get(name.as_str())
                    .map(|_| {
                        // Speed is computed during update(); expose the last-known global speed
                        // proportionally (accurate per-interface tracking is in prev_iface)
                        // For per-interface display we use the delta from the previous stored values
                        let (prev_rx, prev_tx) = self
                            .network_tracker
                            .prev_iface
                            .get(name.as_str())
                            .copied()
                            .unwrap_or((0, 0));
                        let elapsed = self.last_refresh_time.elapsed().as_secs_f64().max(0.1);
                        let rx_spd = if rx >= prev_rx {
                            ((rx - prev_rx) as f64 / elapsed) as u64
                        } else {
                            0
                        };
                        let tx_spd = if tx >= prev_tx {
                            ((tx - prev_tx) as f64 / elapsed) as u64
                        } else {
                            0
                        };
                        (rx_spd, tx_spd)
                    })
                    .unwrap_or((0, 0));

                InterfaceHarvest {
                    name: name.to_string(),
                    rx_bytes: rx,
                    tx_bytes: tx,
                    rx_speed: iface_rx_speed,
                    tx_speed: iface_tx_speed,
                }
            })
            .collect();

        NetworkHarvest {
            total_rx_bytes,
            total_tx_bytes,
            current_rx_speed: self.network_tracker.current_rx_speed,
            current_tx_speed: self.network_tracker.current_tx_speed,
            interfaces,
        }
    }

    pub fn get_disks(&self) -> Vec<DiskHarvest> {
        self.os_engine
            .disks
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let avail = disk.available_space();
                let used = total.saturating_sub(avail);
                let pct = if total > 0 {
                    (used as f32 / total as f32) * 100.0
                } else {
                    0.0
                };

                DiskHarvest {
                    name: disk.name().to_string_lossy().into_owned(),
                    mount_point: disk.mount_point().to_string_lossy().into_owned(),
                    file_system: disk.file_system().to_string_lossy().into_owned(),
                    total_bytes: total,
                    available_bytes: avail,
                    used_bytes: used,
                    used_percent: pct,
                }
            })
            .collect()
    }

    pub fn send_signal(&mut self, pid: u32, signal: ProcessSignal) -> Result<(), String> {
        send_signal_to_pid(pid, signal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_core_initialization() {
        let mut core = SystemCore::new();
        core.refresh();

        let procs = core.get_processes();
        assert!(!procs.is_empty(), "Should harvest at least 1 process");

        let cpu = core.get_cpu();
        assert!(cpu.core_count > 0, "Core count should be > 0");

        let mem = core.get_memory();
        assert!(mem.total_bytes > 0, "Total memory should be > 0");

        let disks = core.get_disks();
        assert!(!disks.is_empty(), "Should detect at least 1 disk/mount");

        let tree = build_process_tree(&procs, "");
        assert!(!tree.is_empty(), "Process tree should not be empty");
    }

    #[test]
    fn test_ports_scanning() {
        let ports = scan_listening_ports();
        // Just verify it runs without crashing
        println!("Detected {} open listening ports", ports.len());
    }
}
