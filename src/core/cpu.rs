use std::collections::VecDeque;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CpuHarvest {
    pub model: String,
    pub core_count: usize,
    pub global_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub load_avg: (f64, f64, f64),
}

pub struct CpuTracker {
    pub history_capacity: usize,
    pub global_history: VecDeque<f64>,
    pub per_core_history: Vec<VecDeque<f64>>,
}

impl CpuTracker {
    pub fn new(capacity: usize) -> Self {
        let mut global_history = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            global_history.push_back(0.0);
        }

        Self {
            history_capacity: capacity,
            global_history,
            per_core_history: Vec::new(),
        }
    }

    pub fn update(&mut self, global_cpu: f32, per_core: &[f32]) {
        self.global_history.pop_front();
        self.global_history.push_back(global_cpu.min(100.0) as f64);

        if self.per_core_history.len() != per_core.len() {
            self.per_core_history.clear();
            for _ in 0..per_core.len() {
                let mut q = VecDeque::with_capacity(self.history_capacity);
                for _ in 0..self.history_capacity {
                    q.push_back(0.0);
                }
                self.per_core_history.push(q);
            }
        }

        for (i, &usage) in per_core.iter().enumerate() {
            if let Some(history) = self.per_core_history.get_mut(i) {
                history.pop_front();
                history.push_back(usage.min(100.0) as f64);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_tracker_initialization() {
        let tracker = CpuTracker::new(10);
        assert_eq!(tracker.history_capacity, 10);
        assert_eq!(tracker.global_history.len(), 10);
        assert!(tracker.global_history.iter().all(|&v| v == 0.0));
        assert!(tracker.per_core_history.is_empty());
    }

    #[test]
    fn test_cpu_tracker_update_and_capping() {
        let mut tracker = CpuTracker::new(5);
        let per_core = vec![25.0, 50.0, 75.0, 120.0]; // 120.0 should be capped to 100.0

        tracker.update(45.5, &per_core);

        assert_eq!(tracker.global_history.len(), 5);
        assert_eq!(*tracker.global_history.back().unwrap(), 45.5);

        assert_eq!(tracker.per_core_history.len(), 4);
        assert_eq!(*tracker.per_core_history[0].back().unwrap(), 25.0);
        assert_eq!(*tracker.per_core_history[1].back().unwrap(), 50.0);
        assert_eq!(*tracker.per_core_history[2].back().unwrap(), 75.0);
        assert_eq!(*tracker.per_core_history[3].back().unwrap(), 100.0); // Capped at 100.0

        // Test subsequent update shifts history
        tracker.update(80.0, &vec![10.0, 20.0, 30.0, 40.0]);
        assert_eq!(tracker.global_history.len(), 5);
        assert_eq!(*tracker.global_history.back().unwrap(), 80.0);
        assert_eq!(*tracker.per_core_history[3].back().unwrap(), 40.0);
    }
}
