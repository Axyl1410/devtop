use std::collections::VecDeque;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MemoryHarvest {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub cache_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub used_percent: f32,
    pub swap_used_percent: f32,
    pub cache_percent: f32,
}

pub struct MemoryTracker {
    /// RAM used history — stored in bytes (like bottom), not percent.
    /// Using bytes allows the Y-axis to show GiB/MiB labels instead of %.
    pub ram_history: VecDeque<f64>,
    /// Swap used history — stored in bytes.
    pub swap_history: VecDeque<f64>,
    /// Cache/buffer history — stored in bytes.
    pub cache_history: VecDeque<f64>,
    /// Total RAM in bytes (constant across samples, used for Y-axis bounds).
    pub total_bytes: u64,
    /// Total Swap in bytes.
    pub swap_total_bytes: u64,
}

impl MemoryTracker {
    pub fn new(capacity: usize) -> Self {
        let mut ram_history = VecDeque::with_capacity(capacity);
        let mut swap_history = VecDeque::with_capacity(capacity);
        let mut cache_history = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            ram_history.push_back(0.0);
            swap_history.push_back(0.0);
            cache_history.push_back(0.0);
        }

        Self {
            ram_history,
            swap_history,
            cache_history,
            total_bytes: 0,
            swap_total_bytes: 0,
        }
    }

    /// Update history using absolute byte values — matching bottom's approach.
    /// The chart Y-axis will be in GiB/MiB, labels derived from total_bytes.
    pub fn update(
        &mut self,
        used_bytes: u64,
        swap_used_bytes: u64,
        cache_bytes: u64,
        total_bytes: u64,
        swap_total_bytes: u64,
        // Still kept for % display in text labels:
        _ram_percent: f32,
        _swap_percent: f32,
        _cache_percent: f32,
    ) {
        self.total_bytes = total_bytes;
        self.swap_total_bytes = swap_total_bytes;

        self.ram_history.pop_front();
        self.ram_history.push_back(used_bytes as f64);

        self.swap_history.pop_front();
        self.swap_history.push_back(swap_used_bytes as f64);

        self.cache_history.pop_front();
        self.cache_history.push_back(cache_bytes as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_initialization() {
        let tracker = MemoryTracker::new(10);
        assert_eq!(tracker.ram_history.len(), 10);
        assert_eq!(tracker.swap_history.len(), 10);
        assert_eq!(tracker.cache_history.len(), 10);
        assert_eq!(tracker.total_bytes, 0);
        assert_eq!(tracker.swap_total_bytes, 0);
    }

    #[test]
    fn test_memory_tracker_update() {
        let mut tracker = MemoryTracker::new(3);

        let total_ram = 16 * 1024 * 1024 * 1024; // 16 GiB
        let used_ram = 8 * 1024 * 1024 * 1024;   // 8 GiB
        let cache_ram = 4 * 1024 * 1024 * 1024;  // 4 GiB
        let swap_total = 8 * 1024 * 1024 * 1024; // 8 GiB
        let swap_used = 1 * 1024 * 1024 * 1024;  // 1 GiB

        tracker.update(used_ram, swap_used, cache_ram, total_ram, swap_total, 50.0, 12.5, 25.0);

        assert_eq!(tracker.total_bytes, total_ram);
        assert_eq!(tracker.swap_total_bytes, swap_total);
        assert_eq!(tracker.ram_history.len(), 3);
        assert_eq!(*tracker.ram_history.back().unwrap(), used_ram as f64);
        assert_eq!(*tracker.swap_history.back().unwrap(), swap_used as f64);
        assert_eq!(*tracker.cache_history.back().unwrap(), cache_ram as f64);
    }
}
