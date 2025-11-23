//! Real-time metrics collection and aggregation
//! 
//! This module provides high-performance metrics collection for monitoring
//! packet injection rates, success rates, and system performance.

use chrono::{DateTime, Utc};
use crossbeam::queue::SegQueue;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

/// Real-time performance metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    /// Total packets injected
    pub packets_injected: u64,
    
    /// Packets injected in the last second
    pub packets_per_second: u64,
    
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    
    /// Total bytes transmitted
    pub bytes_transmitted: u64,
    
    /// Current channel utilization (0.0 - 1.0)
    pub channel_utilization: f64,
    
    /// Number of active targets
    pub active_targets: usize,
    
    /// Average injection latency (microseconds)
    pub avg_latency_us: u64,
    
    /// Peak packets per second
    pub peak_pps: u64,
    
    /// Timestamp of last update
    pub last_update: DateTime<Utc>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            packets_injected: 0,
            packets_per_second: 0,
            success_rate: 0.0,
            bytes_transmitted: 0,
            channel_utilization: 0.0,
            active_targets: 0,
            avg_latency_us: 0,
            peak_pps: 0,
            last_update: Utc::now(),
        }
    }
}

/// High-performance metrics collector
pub struct MetricsCollector {
    /// Total packets injected (atomic counter)
    packets_injected: AtomicU64,
    
    /// Successful injections (atomic counter)
    successful_injections: AtomicU64,
    
    /// Total bytes transmitted (atomic counter)
    bytes_transmitted: AtomicU64,
    
    /// Current active targets (atomic counter)
    active_targets: AtomicUsize,
    
    /// Sliding window for PPS calculation
    packet_timestamps: Arc<SegQueue<Instant>>,
    
    /// Latency measurements
    latency_samples: Arc<SegQueue<Duration>>,
    
    /// Channel utilization samples
    channel_samples: Arc<SegQueue<f64>>,
    
    /// Last metrics snapshot
    last_metrics: RwLock<Metrics>,
    
    /// Window size for moving averages
    window_size: usize,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(window_size: usize) -> Self {
        Self {
            packets_injected: AtomicU64::new(0),
            successful_injections: AtomicU64::new(0),
            bytes_transmitted: AtomicU64::new(0),
            active_targets: AtomicUsize::new(0),
            packet_timestamps: Arc::new(SegQueue::new()),
            latency_samples: Arc::new(SegQueue::new()),
            channel_samples: Arc::new(SegQueue::new()),
            last_metrics: RwLock::new(Metrics::default()),
            window_size,
        }
    }
    
    /// Record a packet injection attempt
    pub fn record_injection(&self, bytes: usize, success: bool, latency: Duration) {
        self.packets_injected.fetch_add(1, Ordering::Relaxed);
        self.bytes_transmitted.fetch_add(bytes as u64, Ordering::Relaxed);
        
        if success {
            self.successful_injections.fetch_add(1, Ordering::Relaxed);
        }
        
        // Record timestamp for PPS calculation
        self.packet_timestamps.push(Instant::now());
        
        // Record latency
        self.latency_samples.push(latency);
        
        debug!("Recorded injection: {} bytes, success: {}, latency: {:?}", bytes, success, latency);
    }
    
    /// Record channel utilization sample
    pub fn record_channel_utilization(&self, utilization: f64) {
        self.channel_samples.push(utilization.clamp(0.0, 1.0));
    }
    
    /// Update active target count
    pub fn set_active_targets(&self, count: usize) {
        self.active_targets.store(count, Ordering::Relaxed);
    }
    
    /// Calculate current metrics
    pub fn calculate_metrics(&self) -> Metrics {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);
        
        // Clean old timestamps and count recent packets
        let mut recent_packets = 0;
        let mut timestamps_to_keep = Vec::new();
        
        while let Some(timestamp) = self.packet_timestamps.pop() {
            if timestamp >= one_second_ago {
                recent_packets += 1;
                timestamps_to_keep.push(timestamp);
            }
        }
        
        // Put back the recent timestamps
        for timestamp in timestamps_to_keep {
            self.packet_timestamps.push(timestamp);
        }
        
        // Calculate average latency
        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0;
        let mut latency_samples_to_keep = Vec::new();
        
        while let Some(latency) = self.latency_samples.pop() {
            total_latency += latency;
            latency_count += 1;
            latency_samples_to_keep.push(latency);
        }
        
        // Keep only the most recent samples
        let latency_samples_to_keep: Vec<_> = latency_samples_to_keep
            .into_iter()
            .rev()
            .take(self.window_size)
            .collect();
        
        for latency in latency_samples_to_keep.iter().rev() {
            self.latency_samples.push(*latency);
        }
        
        let avg_latency_us = if latency_count > 0 {
            (total_latency / latency_count).as_micros() as u64
        } else {
            0
        };
        
        // Calculate average channel utilization
        let mut total_utilization = 0.0;
        let mut utilization_count = 0;
        let mut samples_to_keep = Vec::new();
        
        while let Some(utilization) = self.channel_samples.pop() {
            total_utilization += utilization;
            utilization_count += 1;
            samples_to_keep.push(utilization);
        }
        
        // Keep only the most recent samples
        let samples_to_keep: Vec<_> = samples_to_keep
            .into_iter()
            .rev()
            .take(self.window_size)
            .collect();
        
        for sample in samples_to_keep.iter().rev() {
            self.channel_samples.push(*sample);
        }
        
        let avg_channel_utilization = if utilization_count > 0 {
            total_utilization / utilization_count as f64
        } else {
            0.0
        };
        
        // Calculate success rate
        let total_packets = self.packets_injected.load(Ordering::Relaxed);
        let successful_packets = self.successful_injections.load(Ordering::Relaxed);
        let success_rate = if total_packets > 0 {
            successful_packets as f64 / total_packets as f64
        } else {
            0.0
        };
        
        // Get current metrics and update peak PPS
        let mut current_metrics = self.last_metrics.read().clone();
        let peak_pps = current_metrics.peak_pps.max(recent_packets);
        
        let new_metrics = Metrics {
            packets_injected: total_packets,
            packets_per_second: recent_packets,
            success_rate,
            bytes_transmitted: self.bytes_transmitted.load(Ordering::Relaxed),
            channel_utilization: avg_channel_utilization,
            active_targets: self.active_targets.load(Ordering::Relaxed),
            avg_latency_us,
            peak_pps,
            last_update: Utc::now(),
        };
        
        // Update the cached metrics
        *self.last_metrics.write() = new_metrics.clone();
        
        new_metrics
    }
    
    /// Get the last calculated metrics (fast path)
    pub fn get_metrics(&self) -> Metrics {
        self.last_metrics.read().clone()
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.packets_injected.store(0, Ordering::Relaxed);
        self.successful_injections.store(0, Ordering::Relaxed);
        self.bytes_transmitted.store(0, Ordering::Relaxed);
        self.active_targets.store(0, Ordering::Relaxed);
        
        // Clear all queues
        while self.packet_timestamps.pop().is_some() {}
        while self.latency_samples.pop().is_some() {}
        while self.channel_samples.pop().is_some() {}
        
        *self.last_metrics.write() = Metrics::default();
    }
}

/// Metrics for a specific target
#[derive(Debug, Clone)]
pub struct TargetMetrics {
    pub mac_address: MacAddress,
    pub packets_sent: u64,
    pub success_rate: f64,
    pub last_seen: DateTime<Utc>,
}

/// Per-target metrics collector
pub struct TargetMetricsCollector {
    targets: Arc<RwLock<std::collections::HashMap<MacAddress, TargetMetrics>>>,
}

impl TargetMetricsCollector {
    pub fn new() -> Self {
        Self {
            targets: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    pub fn record_target_activity(&self, mac: MacAddress, success: bool) {
        let mut targets = self.targets.write();
        let entry = targets.entry(mac).or_insert_with(|| TargetMetrics {
            mac_address: mac,
            packets_sent: 0,
            success_rate: 0.0,
            last_seen: Utc::now(),
        });
        
        entry.packets_sent += 1;
        entry.last_seen = Utc::now();
        
        // Simple success rate calculation
        if success {
            entry.success_rate = (entry.success_rate * 0.9) + (1.0 * 0.1);
        } else {
            entry.success_rate = (entry.success_rate * 0.9) + (0.0 * 0.1);
        }
    }
    
    pub fn get_target_metrics(&self, mac: MacAddress) -> Option<TargetMetrics> {
        self.targets.read().get(&mac).cloned()
    }
    
    pub fn get_all_targets(&self) -> Vec<TargetMetrics> {
        self.targets.read().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new(100);
        
        // Record some injections
        for i in 0..10 {
            collector.record_injection(100, i % 2 == 0, Duration::from_micros(100 + i));
        }
        
        let metrics = collector.calculate_metrics();
        
        assert_eq!(metrics.packets_injected, 10);
        assert_eq!(metrics.bytes_transmitted, 550); // 100*10 + 45*10 (sum of 0-9)
        assert!((metrics.success_rate - 0.5).abs() < 0.1);
    }
    
    #[test]
    fn test_target_metrics() {
        let collector = TargetMetricsCollector::new();
        let mac = MacAddress::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        
        collector.record_target_activity(mac, true);
        collector.record_target_activity(mac, true);
        collector.record_target_activity(mac, false);
        
        let metrics = collector.get_target_metrics(mac).expect("Should have metrics");
        assert_eq!(metrics.packets_sent, 3);
        assert!(metrics.success_rate > 0.6); // Should be around 0.67
    }
}