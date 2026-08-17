use crate::core::{
    PortBinding, ProcessHarvest, ProcessSignal, ProcessTreeItem, SystemCore, build_process_tree,
};
use std::time::{Duration, Instant};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ActiveTab {
    Overview = 0,
    Processes = 1,
    Ports = 2,
    StorageNetwork = 3,
    Help = 4,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
    User,
}

pub struct App {
    pub core: SystemCore,
    pub active_tab: ActiveTab,
    pub selected_process: usize,
    pub selected_port: usize,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
    pub tree_mode: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub show_kill_confirm: bool,
    pub kill_target: Option<(u32, String)>,
    pub selected_signal_idx: usize,
    pub show_process_detail: bool,
    pub selected_detail_pid: Option<u32>,
    pub refresh_rate_ms: u64,
    pub last_refresh: Instant,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub scroll_offset: usize,
    pub port_scroll_offset: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            core: SystemCore::new(),
            active_tab: ActiveTab::Overview,
            selected_process: 0,
            selected_port: 0,
            sort_by: SortBy::Cpu,
            sort_ascending: false,
            tree_mode: false,
            search_mode: false,
            search_query: String::new(),
            show_kill_confirm: false,
            kill_target: None,
            selected_signal_idx: 0,
            show_process_detail: false,
            selected_detail_pid: None,
            refresh_rate_ms: 1000,
            last_refresh: Instant::now(),
            status_message: Some((
                "Welcome to devtop. Press '?' for help, [Enter] for process detail.".to_string(),
                Instant::now(),
            )),
            should_quit: false,
            scroll_offset: 0,
            port_scroll_offset: 0,
        }
    }

    pub fn on_tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_millis(self.refresh_rate_ms) {
            self.core.refresh();
            self.last_refresh = Instant::now();
        }

        if let Some((_, time)) = self.status_message {
            if time.elapsed() > Duration::from_secs(4) {
                self.status_message = None;
            }
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    pub fn next_tab(&mut self) {
        if self.show_process_detail {
            self.show_process_detail = false;
        }
        self.active_tab = match self.active_tab {
            ActiveTab::Overview => ActiveTab::Processes,
            ActiveTab::Processes => ActiveTab::Ports,
            ActiveTab::Ports => ActiveTab::StorageNetwork,
            ActiveTab::StorageNetwork => ActiveTab::Help,
            ActiveTab::Help => ActiveTab::Overview,
        };
    }

    pub fn prev_tab(&mut self) {
        if self.show_process_detail {
            self.show_process_detail = false;
        }
        self.active_tab = match self.active_tab {
            ActiveTab::Overview => ActiveTab::Help,
            ActiveTab::Processes => ActiveTab::Overview,
            ActiveTab::Ports => ActiveTab::Processes,
            ActiveTab::StorageNetwork => ActiveTab::Ports,
            ActiveTab::Help => ActiveTab::StorageNetwork,
        };
    }

    pub fn filtered_sorted_processes(&self) -> Vec<ProcessHarvest> {
        let mut list = self.core.get_processes();

        // Filter
        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            list.retain(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.cmd.to_lowercase().contains(&q)
                    || p.pid.to_string().contains(&q)
                    || p.user.to_lowercase().contains(&q)
                    || p.ports.iter().any(|port| port.to_string().contains(&q))
            });
        }

        // Sort
        list.sort_by(|a, b| {
            let ordering = match self.sort_by {
                SortBy::Cpu => b
                    .cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                SortBy::Memory => b.memory_bytes.cmp(&a.memory_bytes),
                SortBy::Pid => a.pid.cmp(&b.pid),
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::User => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
            };
            if self.sort_ascending {
                ordering.reverse()
            } else {
                ordering
            }
        });

        list
    }

    pub fn tree_processes(&self) -> Vec<ProcessTreeItem> {
        let procs = self.core.get_processes();
        build_process_tree(&procs, &self.search_query)
    }

    pub fn filtered_ports(&self) -> Vec<PortBinding> {
        let mut ports = self.core.get_ports();
        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            ports.retain(|p| {
                p.port.to_string().contains(&q)
                    || p.protocol.to_lowercase().contains(&q)
                    || p.ip.to_lowercase().contains(&q)
                    || p.pid
                        .map(|pid| pid.to_string().contains(&q))
                        .unwrap_or(false)
                    || p.process_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&q))
                        .unwrap_or(false)
                    || p.cwd
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&q))
                        .unwrap_or(false)
            });
        }
        ports
    }

    pub fn next_item(&mut self) {
        match self.active_tab {
            ActiveTab::Processes => {
                let len = if self.tree_mode {
                    self.tree_processes().len()
                } else {
                    self.filtered_sorted_processes().len()
                };
                if len > 0 && self.selected_process + 1 < len {
                    self.selected_process += 1;
                }
            }
            ActiveTab::Ports => {
                let len = self.filtered_ports().len();
                if len > 0 && self.selected_port + 1 < len {
                    self.selected_port += 1;
                }
            }
            _ => {}
        }
    }

    pub fn prev_item(&mut self) {
        match self.active_tab {
            ActiveTab::Processes => {
                if self.selected_process > 0 {
                    self.selected_process -= 1;
                }
            }
            ActiveTab::Ports => {
                if self.selected_port > 0 {
                    self.selected_port -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        match self.active_tab {
            ActiveTab::Processes => {
                let len = if self.tree_mode {
                    self.tree_processes().len()
                } else {
                    self.filtered_sorted_processes().len()
                };
                if len > 0 {
                    self.selected_process = (self.selected_process + page_size).min(len - 1);
                }
            }
            ActiveTab::Ports => {
                let len = self.filtered_ports().len();
                if len > 0 {
                    self.selected_port = (self.selected_port + page_size).min(len - 1);
                }
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        match self.active_tab {
            ActiveTab::Processes => {
                self.selected_process = self.selected_process.saturating_sub(page_size);
            }
            ActiveTab::Ports => {
                self.selected_port = self.selected_port.saturating_sub(page_size);
            }
            _ => {}
        }
    }

    pub fn toggle_sort(&mut self, sort: SortBy) {
        if self.sort_by == sort {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_by = sort;
            self.sort_ascending = match sort {
                SortBy::Pid | SortBy::Name | SortBy::User => true,
                SortBy::Cpu | SortBy::Memory => false,
            };
        }
        self.set_status(&format!(
            "Sorted by {:?} ({})",
            match self.sort_by {
                SortBy::Cpu => "CPU%",
                SortBy::Memory => "Memory",
                SortBy::Pid => "PID",
                SortBy::Name => "Name",
                SortBy::User => "User",
            },
            if self.sort_ascending { "Asc" } else { "Desc" }
        ));
    }

    pub fn toggle_tree_mode(&mut self) {
        self.tree_mode = !self.tree_mode;
        self.selected_process = 0;
        self.scroll_offset = 0;
        self.set_status(if self.tree_mode {
            "Process tree mode enabled"
        } else {
            "Process flat table mode enabled"
        });
    }

    pub fn open_process_detail(&mut self) {
        match self.active_tab {
            ActiveTab::Processes => {
                if self.tree_mode {
                    let tree = self.tree_processes();
                    if let Some(item) = tree.get(self.selected_process) {
                        self.selected_detail_pid = Some(item.process.pid);
                        self.show_process_detail = true;
                    }
                } else {
                    let procs = self.filtered_sorted_processes();
                    if let Some(item) = procs.get(self.selected_process) {
                        self.selected_detail_pid = Some(item.pid);
                        self.show_process_detail = true;
                    }
                }
            }
            ActiveTab::Ports => {
                let ports = self.filtered_ports();
                if let Some(item) = ports.get(self.selected_port) {
                    if let Some(pid) = item.pid {
                        self.selected_detail_pid = Some(pid);
                        self.show_process_detail = true;
                    } else {
                        self.set_status(&format!(
                            "Port :{} has no associated PID (permission needed)",
                            item.port
                        ));
                    }
                }
            }
            ActiveTab::Overview => {
                let procs = self.filtered_sorted_processes();
                if let Some(item) = procs.first() {
                    self.selected_detail_pid = Some(item.pid);
                    self.show_process_detail = true;
                }
            }
            _ => {}
        }
    }

    pub fn close_process_detail(&mut self) {
        self.show_process_detail = false;
    }

    pub fn request_kill_selected(&mut self) {
        self.selected_signal_idx = 0; // Default to SIGTERM (15) for safe graceful termination

        if self.show_process_detail {
            if let Some(pid) = self.selected_detail_pid {
                if let Some(proc) = self.core.get_process_by_pid(pid) {
                    self.kill_target = Some((proc.pid, proc.name));
                    self.show_kill_confirm = true;
                }
            }
            return;
        }

        match self.active_tab {
            ActiveTab::Processes => {
                if self.tree_mode {
                    let tree = self.tree_processes();
                    if let Some(item) = tree.get(self.selected_process) {
                        self.kill_target = Some((item.process.pid, item.process.name.clone()));
                        self.show_kill_confirm = true;
                    }
                } else {
                    let procs = self.filtered_sorted_processes();
                    if let Some(target) = procs.get(self.selected_process) {
                        self.kill_target = Some((target.pid, target.name.clone()));
                        self.show_kill_confirm = true;
                    }
                }
            }
            ActiveTab::Ports => {
                let ports = self.filtered_ports();
                if let Some(target) = ports.get(self.selected_port) {
                    if let Some(pid) = target.pid {
                        let name = target
                            .process_name
                            .clone()
                            .unwrap_or_else(|| format!("Port :{}", target.port));
                        self.kill_target = Some((pid, name));
                        self.show_kill_confirm = true;
                    } else {
                        self.set_status(&format!(
                            "Cannot terminate Port :{}: PID unknown",
                            target.port
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn next_signal(&mut self) {
        self.selected_signal_idx = (self.selected_signal_idx + 1) % ProcessSignal::ALL.len();
    }

    pub fn prev_signal(&mut self) {
        if self.selected_signal_idx == 0 {
            self.selected_signal_idx = ProcessSignal::ALL.len() - 1;
        } else {
            self.selected_signal_idx -= 1;
        }
    }

    pub fn select_signal_by_index(&mut self, idx: usize) {
        if idx < ProcessSignal::ALL.len() {
            self.selected_signal_idx = idx;
        }
    }

    pub fn confirm_kill(&mut self) {
        if let Some((pid, name)) = self.kill_target.take() {
            let signal = ProcessSignal::ALL
                .get(self.selected_signal_idx)
                .copied()
                .unwrap_or(ProcessSignal::Term);

            match self.core.send_signal(pid, signal) {
                Ok(()) => {
                    self.set_status(&format!("Sent {} to {} (PID {})", signal.name(), name, pid));
                }
                Err(err) => {
                    self.set_status(&format!("Failed to send signal to {} (PID {}): {}", name, pid, err));
                }
            }
        }
        self.show_kill_confirm = false;
        self.core.refresh();
    }

    pub fn cancel_kill(&mut self) {
        self.kill_target = None;
        self.show_kill_confirm = false;
        self.set_status("Signal transmission canceled");
    }

    pub fn increase_refresh_rate(&mut self) {
        if self.refresh_rate_ms < 5000 {
            self.refresh_rate_ms += 500;
            self.set_status(&format!("Refresh interval: {}ms", self.refresh_rate_ms));
        }
    }

    pub fn decrease_refresh_rate(&mut self) {
        if self.refresh_rate_ms > 250 {
            self.refresh_rate_ms -= 250;
            self.set_status(&format!("Refresh interval: {}ms", self.refresh_rate_ms));
        }
    }
}
