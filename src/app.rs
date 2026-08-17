use crate::system::{ProcessItem, SystemStats};
use std::time::{Duration, Instant};

#[derive(PartialEq, Clone, Copy)]
pub enum ActiveTab {
    Overview = 0,
    Processes = 1,
    StorageNetwork = 2,
    Help = 3,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

pub struct App {
    pub stats: SystemStats,
    pub active_tab: ActiveTab,
    pub selected_process: usize,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub show_kill_confirm: bool,
    pub kill_target: Option<(u32, String)>,
    pub refresh_rate_ms: u64,
    pub last_refresh: Instant,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
    pub scroll_offset: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            stats: SystemStats::new(),
            active_tab: ActiveTab::Overview,
            selected_process: 0,
            sort_by: SortBy::Cpu,
            sort_ascending: false,
            search_mode: false,
            search_query: String::new(),
            show_kill_confirm: false,
            kill_target: None,
            refresh_rate_ms: 1000,
            last_refresh: Instant::now(),
            status_message: Some((
                "Welcome to devtop! Press '?' or '4' for help.".to_string(),
                Instant::now(),
            )),
            should_quit: false,
            scroll_offset: 0,
        }
    }

    pub fn on_tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_millis(self.refresh_rate_ms) {
            self.stats.refresh();
            self.last_refresh = Instant::now();
        }

        // Clear status message after 4 seconds
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
        self.active_tab = match self.active_tab {
            ActiveTab::Overview => ActiveTab::Processes,
            ActiveTab::Processes => ActiveTab::StorageNetwork,
            ActiveTab::StorageNetwork => ActiveTab::Help,
            ActiveTab::Help => ActiveTab::Overview,
        };
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Overview => ActiveTab::Help,
            ActiveTab::Processes => ActiveTab::Overview,
            ActiveTab::StorageNetwork => ActiveTab::Processes,
            ActiveTab::Help => ActiveTab::StorageNetwork,
        };
    }

    pub fn filtered_sorted_processes(&self) -> Vec<ProcessItem> {
        let mut list = self.stats.get_processes();

        // Filter
        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            list.retain(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.cmd.to_lowercase().contains(&q)
                    || p.pid.to_string().contains(&q)
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
            };
            if self.sort_ascending {
                ordering.reverse()
            } else {
                ordering
            }
        });

        list
    }

    pub fn next_process(&mut self, list_len: usize) {
        if list_len > 0 {
            if self.selected_process + 1 < list_len {
                self.selected_process += 1;
            }
        }
    }

    pub fn prev_process(&mut self) {
        if self.selected_process > 0 {
            self.selected_process -= 1;
        }
    }

    pub fn toggle_sort(&mut self, sort: SortBy) {
        if self.sort_by == sort {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_by = sort;
            self.sort_ascending = match sort {
                SortBy::Pid | SortBy::Name => true,
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
            },
            if self.sort_ascending { "Asc" } else { "Desc" }
        ));
    }

    pub fn request_kill_selected(&mut self) {
        let procs = self.filtered_sorted_processes();
        if let Some(target) = procs.get(self.selected_process) {
            self.kill_target = Some((target.pid, target.name.clone()));
            self.show_kill_confirm = true;
        }
    }

    pub fn confirm_kill(&mut self) {
        if let Some((pid, name)) = self.kill_target.take() {
            if self.stats.kill_process(pid) {
                self.set_status(&format!("Successfully sent KILL signal to {} (PID {})", name, pid));
            } else {
                self.set_status(&format!("Failed to kill {} (PID {}): Permission denied or process gone", name, pid));
            }
        }
        self.show_kill_confirm = false;
        self.stats.refresh();
    }

    pub fn cancel_kill(&mut self) {
        self.kill_target = None;
        self.show_kill_confirm = false;
        self.set_status("Kill action canceled");
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
