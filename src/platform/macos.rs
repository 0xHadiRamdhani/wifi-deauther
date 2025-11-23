//! macOS platform implementation

use super::{Platform, PlatformCapabilities};
use crate::Result;

pub struct MacOSPlatform;

impl MacOSPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Platform for MacOSPlatform {
    fn name(&self) -> &str {
        "macOS"
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            raw_socket_support: true,
            monitor_mode_support: false,
            bpf_support: true,
            netlink_support: false,
            winpcap_support: false,
        }
    }
}