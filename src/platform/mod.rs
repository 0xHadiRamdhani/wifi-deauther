//! Platform-specific implementations
//! 
//! This module provides platform-specific functionality for
//! Linux, Windows, and macOS systems.

pub mod linux;
pub mod windows;
pub mod macos;

pub use linux::LinuxPlatform;
pub use windows::WindowsPlatform;
pub use macos::MacOSPlatform;

use crate::Result;

/// Platform trait for cross-platform operations
pub trait Platform {
    /// Get platform name
    fn name(&self) -> &str;
    
    /// Check if platform is supported
    fn is_supported(&self) -> bool;
    
    /// Get platform-specific capabilities
    fn capabilities(&self) -> PlatformCapabilities;
}

/// Platform capabilities
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub raw_socket_support: bool,
    pub monitor_mode_support: bool,
    pub bpf_support: bool,
    pub netlink_support: bool,
    pub winpcap_support: bool,
}

/// Get current platform
pub fn get_current_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "linux")]
    return Box::new(LinuxPlatform::new());
    
    #[cfg(target_os = "windows")]
    return Box::new(WindowsPlatform::new());
    
    #[cfg(target_os = "macos")]
    return Box::new(MacOSPlatform::new());
    
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    panic!("Unsupported platform");
}