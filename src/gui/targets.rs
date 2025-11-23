//! Target management for GUI

use crate::network::NetworkInterface;
use mac_address::MacAddress;
use std::collections::HashMap;
use tracing::{debug, info};

/// Wi-Fi target information
#[derive(Debug, Clone)]
pub struct Target {
    pub mac_address: MacAddress,
    pub ssid: String,
    pub channel: u8,
    pub signal_strength: i8,
    pub encryption: EncryptionType,
    pub vendor: Option<String>,
    pub last_seen: std::time::SystemTime,
}

/// Encryption type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionType {
    Open,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Unknown,
}

/// Target manager
pub struct TargetManager {
    targets: HashMap<MacAddress, Target>,
    selected_targets: Vec<MacAddress>,
}

impl TargetManager {
    /// Create a new target manager
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            selected_targets: Vec::new(),
        }
    }
    
    /// Add or update a target
    pub fn add_target(&mut self, target: Target) {
        info!("Adding target: {} ({})", target.mac_address, target.ssid);
        self.targets.insert(target.mac_address, target);
    }
    
    /// Remove a target
    pub fn remove_target(&mut self, mac: &MacAddress) -> Option<Target> {
        info!("Removing target: {}", mac);
        self.selected_targets.retain(|m| m != mac);
        self.targets.remove(mac)
    }
    
    /// Get all targets
    pub fn get_targets(&self) -> Vec<&Target> {
        self.targets.values().collect()
    }
    
    /// Get target by MAC address
    pub fn get_target(&self, mac: &MacAddress) -> Option<&Target> {
        self.targets.get(mac)
    }
    
    /// Select a target for attack
    pub fn select_target(&mut self, mac: MacAddress) -> Result<(), String> {
        if !self.targets.contains_key(&mac) {
            return Err("Target not found".to_string());
        }
        
        if !self.selected_targets.contains(&mac) {
            self.selected_targets.push(mac);
            debug!("Selected target: {}", mac);
        }
        
        Ok(())
    }
    
    /// Deselect a target
    pub fn deselect_target(&mut self, mac: &MacAddress) {
        self.selected_targets.retain(|m| m != mac);
        debug!("Deselected target: {}", mac);
    }
    
    /// Get selected targets
    pub fn get_selected_targets(&self) -> Vec<&Target> {
        self.selected_targets
            .iter()
            .filter_map(|mac| self.targets.get(mac))
            .collect()
    }
    
    /// Clear all targets
    pub fn clear_targets(&mut self) {
        info!("Clearing all targets");
        self.targets.clear();
        self.selected_targets.clear();
    }
    
    /// Update target signal strength
    pub fn update_signal(&mut self, mac: MacAddress, signal: i8) {
        if let Some(target) = self.targets.get_mut(&mac) {
            target.signal_strength = signal;
            target.last_seen = std::time::SystemTime::now();
            debug!("Updated signal for {}: {} dBm", mac, signal);
        }
    }
}

impl Default for TargetManager {
    fn default() -> Self {
        Self::new()
    }
}