//! Real-time charting and visualization

use crate::core::Metrics;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use tracing::debug;

/// Chart data point
#[derive(Debug, Clone)]
pub struct ChartPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Real-time metrics chart
pub struct MetricsChart {
    max_points: usize,
    packets_per_second: VecDeque<ChartPoint>,
    success_rate: VecDeque<ChartPoint>,
    channel_utilization: VecDeque<ChartPoint>,
}

impl MetricsChart {
    /// Create a new metrics chart
    pub fn new(max_points: usize) -> Self {
        Self {
            max_points,
            packets_per_second: VecDeque::with_capacity(max_points),
            success_rate: VecDeque::with_capacity(max_points),
            channel_utilization: VecDeque::with_capacity(max_points),
        }
    }
    
    /// Add a new metrics data point
    pub fn add_point(&mut self, metrics: &Metrics) {
        let now = Utc::now();
        
        // Add packets per second
        self.packets_per_second.push_back(ChartPoint {
            timestamp: now,
            value: metrics.packets_per_second as f64,
        });
        
        // Add success rate
        self.success_rate.push_back(ChartPoint {
            timestamp: now,
            value: metrics.success_rate * 100.0,
        });
        
        // Add channel utilization
        self.channel_utilization.push_back(ChartPoint {
            timestamp: now,
            value: metrics.channel_utilization * 100.0,
        });
        
        // Trim old points
        self.trim_old_points();
        
        debug!("Added chart point: PPS={}, Success={:.1}%, Util={:.1}%",
               metrics.packets_per_second, metrics.success_rate * 100.0, metrics.channel_utilization * 100.0);
    }
    
    /// Trim old data points to maintain max size
    fn trim_old_points(&mut self) {
        while self.packets_per_second.len() > self.max_points {
            self.packets_per_second.pop_front();
        }
        while self.success_rate.len() > self.max_points {
            self.success_rate.pop_front();
        }
        while self.channel_utilization.len() > self.max_points {
            self.channel_utilization.pop_front();
        }
    }
    
    /// Get packets per second data
    pub fn get_packets_per_second(&self) -> Vec<(f64, f64)> {
        self.packets_per_second
            .iter()
            .enumerate()
            .map(|(i, point)| (i as f64, point.value))
            .collect()
    }
    
    /// Get success rate data
    pub fn get_success_rate(&self) -> Vec<(f64, f64)> {
        self.success_rate
            .iter()
            .enumerate()
            .map(|(i, point)| (i as f64, point.value))
            .collect()
    }
    
    /// Get channel utilization data
    pub fn get_channel_utilization(&self) -> Vec<(f64, f64)> {
        self.channel_utilization
            .iter()
            .enumerate()
            .map(|(i, point)| (i as f64, point.value))
            .collect()
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.packets_per_second.clear();
        self.success_rate.clear();
        self.channel_utilization.clear();
    }
}

/// Chart configuration
#[derive(Debug, Clone)]
pub struct ChartConfig {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub y_min: f64,
    pub y_max: f64,
    pub color: String,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            title: "Chart".to_string(),
            x_label: "Time".to_string(),
            y_label: "Value".to_string(),
            y_min: 0.0,
            y_max: 100.0,
            color: "#4CAF50".to_string(),
        }
    }
}