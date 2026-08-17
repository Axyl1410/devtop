use std::collections::VecDeque;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InterfaceHarvest {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// Instantaneous receive speed in bytes/sec for this interface
    pub rx_speed: u64,
    /// Instantaneous transmit speed in bytes/sec for this interface
    pub tx_speed: u64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NetworkHarvest {
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub current_rx_speed: u64,
    pub current_tx_speed: u64,
    pub interfaces: Vec<InterfaceHarvest>,
}

pub struct NetworkTracker {
    /// Previous per-interface rx/tx totals to compute per-interface delta speeds
    pub prev_iface: std::collections::HashMap<String, (u64, u64)>,
    pub prev_rx: u64,
    pub prev_tx: u64,
    pub current_rx_speed: u64,
    pub current_tx_speed: u64,
    pub rx_history: VecDeque<f64>,
    pub tx_history: VecDeque<f64>,
}

impl NetworkTracker {
    pub fn new(capacity: usize) -> Self {
        let mut rx_history = VecDeque::with_capacity(capacity);
        let mut tx_history = VecDeque::with_capacity(capacity);
        for _ in 0..capacity {
            rx_history.push_back(0.0);
            tx_history.push_back(0.0);
        }

        Self {
            prev_iface: std::collections::HashMap::new(),
            prev_rx: 0,
            prev_tx: 0,
            current_rx_speed: 0,
            current_tx_speed: 0,
            rx_history,
            tx_history,
        }
    }

    /// Update global network speed based on delta from previous sample.
    /// Returns per-interface speeds for get_network().
    pub fn update(
        &mut self,
        total_rx: u64,
        total_tx: u64,
        delta_secs: f64,
        interfaces_raw: &[(String, u64, u64)], // (name, rx_total, tx_total)
    ) -> Vec<(String, u64, u64, u64, u64)> {
        // Global speed
        if self.prev_rx > 0 && total_rx >= self.prev_rx {
            let diff = total_rx - self.prev_rx;
            self.current_rx_speed = if delta_secs > 0.0 {
                (diff as f64 / delta_secs) as u64
            } else {
                diff
            };
        } else if self.prev_rx == 0 {
            self.current_rx_speed = 0;
        }

        if self.prev_tx > 0 && total_tx >= self.prev_tx {
            let diff = total_tx - self.prev_tx;
            self.current_tx_speed = if delta_secs > 0.0 {
                (diff as f64 / delta_secs) as u64
            } else {
                diff
            };
        } else if self.prev_tx == 0 {
            self.current_tx_speed = 0;
        }

        self.prev_rx = total_rx;
        self.prev_tx = total_tx;

        // Push to rolling history in KB/s (matches bottom's graph scale)
        self.rx_history.pop_front();
        self.rx_history
            .push_back(self.current_rx_speed as f64 / 1024.0);
        self.tx_history.pop_front();
        self.tx_history
            .push_back(self.current_tx_speed as f64 / 1024.0);

        // Per-interface speeds
        let mut result = Vec::with_capacity(interfaces_raw.len());
        for (name, rx, tx) in interfaces_raw {
            let (prev_rx, prev_tx) = self.prev_iface.get(name).copied().unwrap_or((0, 0));
            let iface_rx_speed = if prev_rx > 0 && *rx >= prev_rx && delta_secs > 0.0 {
                ((rx - prev_rx) as f64 / delta_secs) as u64
            } else {
                0
            };
            let iface_tx_speed = if prev_tx > 0 && *tx >= prev_tx && delta_secs > 0.0 {
                ((tx - prev_tx) as f64 / delta_secs) as u64
            } else {
                0
            };
            self.prev_iface.insert(name.clone(), (*rx, *tx));
            result.push((name.clone(), *rx, *tx, iface_rx_speed, iface_tx_speed));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_tracker_initialization() {
        let tracker = NetworkTracker::new(5);
        assert_eq!(tracker.rx_history.len(), 5);
        assert_eq!(tracker.tx_history.len(), 5);
        assert_eq!(tracker.current_rx_speed, 0);
        assert_eq!(tracker.current_tx_speed, 0);
    }

    #[test]
    fn test_network_tracker_speed_computation() {
        let mut tracker = NetworkTracker::new(5);

        let ifaces_1 = vec![
            ("eth0".to_string(), 1000u64, 500u64),
            ("wlan0".to_string(), 2000u64, 1500u64),
        ];

        // First sample seeds previous totals -> speed should be 0
        let res1 = tracker.update(3000, 2000, 1.0, &ifaces_1);
        assert_eq!(tracker.current_rx_speed, 0);
        assert_eq!(tracker.current_tx_speed, 0);
        assert_eq!(res1[0].3, 0); // eth0 rx_speed
        assert_eq!(res1[1].4, 0); // wlan0 tx_speed

        // Second sample after 2.0 seconds
        // eth0 received +2048 bytes (1024 B/s), transmitted +4096 bytes (2048 B/s)
        // wlan0 received +4096 bytes (2048 B/s), transmitted +2048 bytes (1024 B/s)
        // Total RX = 3000 + 6144 = 9144 (+6144 in 2s -> 3072 B/s)
        let ifaces_2 = vec![
            ("eth0".to_string(), 1000 + 2048, 500 + 4096),
            ("wlan0".to_string(), 2000 + 4096, 1500 + 2048),
        ];

        let res2 = tracker.update(9144, 8144, 2.0, &ifaces_2);

        assert_eq!(tracker.current_rx_speed, 3072);
        assert_eq!(tracker.current_tx_speed, 3072);

        // Check per-interface speeds
        assert_eq!(res2[0].0, "eth0");
        assert_eq!(res2[0].3, 1024); // rx speed = 2048 / 2.0 = 1024 B/s
        assert_eq!(res2[0].4, 2048); // tx speed = 4096 / 2.0 = 2048 B/s

        assert_eq!(res2[1].0, "wlan0");
        assert_eq!(res2[1].3, 2048); // rx speed = 4096 / 2.0 = 2048 B/s
        assert_eq!(res2[1].4, 1024); // tx speed = 2048 / 2.0 = 1024 B/s

        // Verify history stored in KB/s
        assert_eq!(*tracker.rx_history.back().unwrap(), 3072.0 / 1024.0); // 3.0 KB/s
    }
}
