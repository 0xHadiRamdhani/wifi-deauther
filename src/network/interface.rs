//! Network interface management for cross-platform Wi-Fi operations
//! 
//! This module provides unified interface for managing network interfaces
//! across Linux, Windows, and macOS platforms.

use crate::{DeauthError, Result};
use mac_address::MacAddress;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// Interface name (e.g., "wlan0", "Wi-Fi")
    pub name: String,
    
    /// Interface index
    pub index: u32,
    
    /// MAC address
    pub mac_address: MacAddress,
    
    /// Interface type (Wi-Fi, Ethernet, etc.)
    pub interface_type: InterfaceType,
    
    /// Current status
    pub status: InterfaceStatus,
    
    /// Supported channels (for Wi-Fi interfaces)
    pub supported_channels: Vec<u8>,
    
    /// Current channel (for Wi-Fi interfaces)
    pub current_channel: Option<u8>,
    
    /// Signal strength (dBm, for Wi-Fi interfaces)
    pub signal_strength: Option<i8>,
    
    /// Platform-specific data
    pub platform_data: PlatformInterfaceData,
}

/// Interface type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    WiFi,
    Ethernet,
    Loopback,
    Other,
}

/// Interface status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceStatus {
    Up,
    Down,
    Unknown,
}

/// Platform-specific interface data
#[derive(Debug, Clone)]
pub enum PlatformInterfaceData {
    Linux(LinuxInterfaceData),
    Windows(WindowsInterfaceData),
    MacOS(MacOSInterfaceData),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct LinuxInterfaceData {
    pub ifindex: u32,
    pub flags: u32,
    pub driver: String,
}

#[derive(Debug, Clone)]
pub struct WindowsInterfaceData {
    pub guid: String,
    pub description: String,
    pub adapter_type: String,
}

#[derive(Debug, Clone)]
pub struct MacOSInterfaceData {
    pub bpf_device: Option<String>,
    pub io_service: String,
}

/// Interface manager for discovering and managing network interfaces
pub struct InterfaceManager {
    interfaces: Arc<std::sync::RwLock<HashMap<String, NetworkInterface>>>,
}

impl InterfaceManager {
    /// Create a new interface manager
    pub fn new() -> Result<Self> {
        let manager = Self {
            interfaces: Arc::new(std::sync::RwLock::new(HashMap::new())),
        };
        
        // Discover interfaces on creation
        manager.discover_interfaces()?;
        
        Ok(manager)
    }
    
    /// Discover all available network interfaces
    pub fn discover_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        info!("Discovering network interfaces");
        
        let interfaces = match platform::get_platform() {
            Platform::Linux => self.discover_linux_interfaces()?,
            Platform::Windows => self.discover_windows_interfaces()?,
            Platform::MacOS => self.discover_macos_interfaces()?,
            Platform::Unknown => return Err(DeauthError::PlatformError("Unsupported platform".to_string())),
        };
        
        // Update internal cache
        {
            let mut cache = self.interfaces.write().unwrap();
            cache.clear();
            for interface in &interfaces {
                cache.insert(interface.name.clone(), interface.clone());
            }
        }
        
        info!("Discovered {} network interfaces", interfaces.len());
        Ok(interfaces)
    }
    
    /// Get all interfaces
    pub fn get_interfaces(&self) -> Vec<NetworkInterface> {
        self.interfaces.read().unwrap().values().cloned().collect()
    }
    
    /// Get Wi-Fi interfaces only
    pub fn get_wifi_interfaces(&self) -> Vec<NetworkInterface> {
        self.interfaces.read().unwrap()
            .values()
            .filter(|iface| iface.interface_type == InterfaceType::WiFi)
            .cloned()
            .collect()
    }
    
    /// Get interface by name
    pub fn get_interface(&self, name: &str) -> Option<NetworkInterface> {
        self.interfaces.read().unwrap().get(name).cloned()
    }
    
    /// Check if interface supports monitor mode
    pub fn supports_monitor_mode(&self, interface: &NetworkInterface) -> Result<bool> {
        match &interface.platform_data {
            PlatformInterfaceData::Linux(data) => {
                self.check_linux_monitor_mode(&interface.name, data)
            }
            PlatformInterfaceData::Windows(_) => {
                // Windows doesn't have traditional monitor mode
                Ok(true)
            }
            PlatformInterfaceData::MacOS(_) => {
                // macOS BPF can capture in monitor mode
                Ok(true)
            }
            PlatformInterfaceData::Unknown => Ok(false),
        }
    }
    
    /// Enable monitor mode on interface (Linux only)
    pub fn enable_monitor_mode(&self, interface: &NetworkInterface) -> Result<()> {
        if interface.interface_type != InterfaceType::WiFi {
            return Err(DeauthError::InterfaceError(
                "Monitor mode only supported on Wi-Fi interfaces".to_string()
            ));
        }
        
        match &interface.platform_data {
            PlatformInterfaceData::Linux(_) => {
                self.enable_linux_monitor_mode(&interface.name)
            }
            PlatformInterfaceData::Windows(_) => {
                Err(DeauthError::PlatformError(
                    "Monitor mode not supported on Windows".to_string()
                ))
            }
            PlatformInterfaceData::MacOS(_) => {
                // macOS uses BPF, no need to enable monitor mode
                Ok(())
            }
            PlatformInterfaceData::Unknown => {
                Err(DeauthError::PlatformError("Unknown platform".to_string()))
            }
        }
    }
    
    /// Linux interface discovery
    fn discover_linux_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        use std::fs;
        use std::path::Path;
        
        let mut interfaces = Vec::new();
        
        // Read network interfaces from /sys/class/net
        let net_path = Path::new("/sys/class/net");
        if !net_path.exists() {
            return Err(DeauthError::PlatformError("/sys/class/net not found".to_string()));
        }
        
        for entry in fs::read_dir(net_path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip loopback
            if name == "lo" {
                continue;
            }
            
            let interface_path = entry.path();
            
            // Get MAC address
            let address_file = interface_path.join("address");
            let mac_address = if address_file.exists() {
                let addr_str = fs::read_to_string(&address_file)?
                    .trim()
                    .to_string();
                MacAddress::from_str(&addr_str)
                    .map_err(|_| DeauthError::InterfaceError("Invalid MAC address".to_string()))?
            } else {
                continue;
            };
            
            // Get interface index
            let ifindex_file = interface_path.join("ifindex");
            let index = if ifindex_file.exists() {
                fs::read_to_string(&ifindex_file)?
                    .trim()
                    .parse::<u32>()
                    .unwrap_or(0)
            } else {
                0
            };
            
            // Check if it's wireless
            let wireless_path = interface_path.join("wireless");
            let interface_type = if wireless_path.exists() {
                InterfaceType::WiFi
            } else {
                InterfaceType::Ethernet
            };
            
            // Get operational status
            let operstate_file = interface_path.join("operstate");
            let status = if operstate_file.exists() {
                match fs::read_to_string(&operstate_file)?.trim() {
                    "up" => InterfaceStatus::Up,
                    "down" => InterfaceStatus::Down,
                    _ => InterfaceStatus::Unknown,
                }
            } else {
                InterfaceStatus::Unknown
            };
            
            // Get driver information
            let device_path = interface_path.join("device");
            let driver = if device_path.exists() {
                if let Ok(driver_link) = fs::read_link(device_path.join("driver")) {
                    driver_link.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };
            
            let interface = NetworkInterface {
                name: name.clone(),
                index,
                mac_address,
                interface_type,
                status,
                supported_channels: Vec::new(), // Will be populated later
                current_channel: None,
                signal_strength: None,
                platform_data: PlatformInterfaceData::Linux(LinuxInterfaceData {
                    ifindex: index,
                    flags: 0, // Will be populated from netlink
                    driver,
                }),
            };
            
            interfaces.push(interface);
        }
        
        Ok(interfaces)
    }
    
    /// Windows interface discovery
    fn discover_windows_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        // This would use Windows APIs through winapi crate
        // For now, return a placeholder
        warn!("Windows interface discovery not yet implemented");
        Ok(Vec::new())
    }
    
    /// macOS interface discovery
    fn discover_macos_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        // This would use IOKit and BSD APIs
        // For now, return a placeholder
        warn!("macOS interface discovery not yet implemented");
        Ok(Vec::new())
    }
    
    /// Check Linux monitor mode support
    fn check_linux_monitor_mode(&self, interface_name: &str, _data: &LinuxInterfaceData) -> Result<bool> {
        // Check if the interface supports monitor mode
        // This would involve checking the driver capabilities
        debug!("Checking monitor mode support for {}", interface_name);
        
        // Placeholder - would need to check driver capabilities
        Ok(true)
    }
    
    /// Enable monitor mode on Linux
    fn enable_linux_monitor_mode(&self, interface_name: &str) -> Result<()> {
        use std::process::Command;
        
        info!("Enabling monitor mode for {}", interface_name);
        
        // Use iw to set monitor mode
        let output = Command::new("iw")
            .args(&[interface_name, "set", "monitor", "fcs"])
            .output()
            .map_err(|e| DeauthError::InterfaceError(format!("Failed to enable monitor mode: {}", e)))?;
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DeauthError::InterfaceError(format!("Monitor mode failed: {}", error)));
        }
        
        info!("Monitor mode enabled for {}", interface_name);
        Ok(())
    }
}

/// Platform detection
mod platform {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Platform {
        Linux,
        Windows,
        MacOS,
        Unknown,
    }
    
    pub fn get_platform() -> Platform {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        return Platform::Unknown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interface_manager_creation() {
        let manager = InterfaceManager::new();
        assert!(manager.is_ok());
    }
    
    #[test]
    fn test_interface_types() {
        assert_eq!(InterfaceType::WiFi, InterfaceType::WiFi);
        assert_ne!(InterfaceType::WiFi, InterfaceType::Ethernet);
    }
    
    #[test]
    fn test_platform_detection() {
        let platform = platform::get_platform();
        
        #[cfg(target_os = "linux")]
        assert_eq!(platform, platform::Platform::Linux);
        
        #[cfg(target_os = "windows")]
        assert_eq!(platform, platform::Platform::Windows);
        
        #[cfg(target_os = "macos")]
        assert_eq!(platform, platform::Platform::MacOS);
    }
}