//! Linux platform implementation

use super::{Platform, PlatformCapabilities};
use crate::Result;

pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for LinuxPlatform {
    fn name(&self) -> &str {
        "Linux"
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            raw_socket_support: true,
            monitor_mode_support: true,
            bpf_support: false,
            netlink_support: true,
            winpcap_support: false,
        }
    }
}