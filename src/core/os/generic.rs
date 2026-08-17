use std::time::{Duration, Instant};
use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind, ProcessesToUpdate,
    RefreshKind, System, Users,
};

pub struct GenericOsEngine {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub users: Users,
    // Throttle slow-refresh tasks like bottom does
    last_slow_refresh: Instant,
}

impl GenericOsEngine {
    pub fn new() -> Self {
        // Use targeted refresh kinds, not refresh_all (expensive)
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything())
                .with_processes(ProcessRefreshKind::everything()),
        );
        // Initial double-sample for CPU delta (sysinfo needs 2 samples)
        sys.refresh_cpu_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_all();
        sys.refresh_memory();
        sys.refresh_processes(ProcessesToUpdate::All, true);

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let users = Users::new_with_refreshed_list();

        Self {
            sys,
            disks,
            networks,
            users,
            last_slow_refresh: Instant::now(),
        }
    }

    /// Hot path: refresh CPU, memory, processes, and networks every tick.
    /// Slow path (disks, users): refresh only every ~60 seconds like bottom.
    pub fn refresh(&mut self) {
        // -- Hot path (every tick) --
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.networks.refresh(true);

        // -- Slow path (every 60 seconds, like bottom's less_routine_tasks) --
        if self.last_slow_refresh.elapsed() >= Duration::from_secs(60) {
            self.disks.refresh(true);
            self.users.refresh();
            self.last_slow_refresh = Instant::now();
        }
    }
}
